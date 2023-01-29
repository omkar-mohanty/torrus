use super::peer_context::PeerContext;
use crate::block::{BlockInfo, BLOCK_SIZE};
use crate::error::TorrusError;
use crate::message::Message;
use crate::peer::session::PeerSession;
use crate::peer::state::{ChokeStatus, ConnectionStatus, Intrest};
use crate::torrent::Context;
use crate::{PeerId, Receiver};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio::{net::TcpStream, sync::mpsc::unbounded_channel};

/// Abstracts over all [`Peer`] operations
pub trait Peer: Send {
    /// Send message to the remote Peer
    fn send(&self, msg: Message) -> Result<()>;
    /// Close all communication with remote Peer
    fn close(self);
}

pub fn new_peer(stream: TcpStream, context: Arc<Context>, peer_id: PeerId) -> super::Peer {
    Box::new(PeerHandle::new(stream, context, peer_id))
}

type Result<T> = std::result::Result<T, crate::error::TorrusError>;

/// The torrent manager handles peer through this struct.
/// Communication mostly takes place by sending messages across a channel
struct PeerHandle {
    /// Context shared by [`Torrent`] and [`PeerHandle`]
    pub peer_context: Arc<PeerContext>,
    /// Receiver for events from the peer
    receiver_join_handle: JoinHandle<Result<()>>,
    /// Sender to send messages to the remote Peer
    sender_join_handle: JoinHandle<Result<()>>,
}

impl Peer for PeerHandle {
    fn send(&self, msg: Message) -> Result<()> {
        if let Err(err) = self.peer_context.sender.send(msg) {
            return Err(TorrusError::new(&err.to_string()));
        } else {
            Ok(())
        }
    }

    fn close(self) {
        self.receiver_join_handle.abort();
        self.sender_join_handle.abort();
        self.peer_context.close_session();
    }
}

impl PeerHandle {
    /// Immediately start the session while the handle is being constructed
    pub fn new(stream: TcpStream, context: Arc<Context>, peer_id: PeerId) -> Self {
        let (msg_send, receiver) = unbounded_channel();

        let (sender, mut peer_session) = PeerSession::new(msg_send);

        let bitfield_len = context.metainfo.total_pieces();

        let peer_session_handle = tokio::spawn(async move {
            if let Err(_) = peer_session.start(stream).await {
                return;
            }
        });

        let peer_context = PeerContext::new(peer_id, sender, bitfield_len, peer_session_handle);

        let peer_context = Arc::new(peer_context);
        let task_peer_context = Arc::clone(&peer_context);

        let receiver_context = Arc::clone(&context);

        let receiver_join_handle = tokio::spawn(async move {
            handle_receiver(receiver, receiver_context, Arc::clone(&task_peer_context)).await?;
            Ok::<(), crate::error::TorrusError>(())
        });

        let sender_context = Arc::clone(&context);

        let sender_peer_context = Arc::clone(&peer_context);

        let sender_join_handle = tokio::spawn(async move {
            message_sender(sender_context, sender_peer_context).await?;

            Ok::<(), crate::error::TorrusError>(())
        });

        Self {
            peer_context,
            receiver_join_handle,
            sender_join_handle,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.receiver_join_handle.is_finished()
    }
}

/// First send [`Message::Interested`] to the Remote client then send
/// the selected message to the remote client. If message selector takes more that 120 seconds
/// that is 2 minutes then send the [`Message::KeepAlive`] message.
async fn message_sender(context: Arc<Context>, peer_context: Arc<PeerContext>) -> Result<()> {
    peer_context.sender.send(Message::Interested)?;

    log::info!("\tmessage_sender:\tsend message Interested");

    loop {
        let fut = select_message(&context, &peer_context);

        let timeout_dur = Duration::from_secs(120);

        match timeout(timeout_dur, fut).await {
            Ok(msg) => {
                let msg = msg?;

                log::debug!("\tclient_side:\tSent {} message", msg);

                peer_context.sender.send(msg)?;
            }

            Err(_) => {
                log::debug!(
                    "\tclient_side:\tselect_message did not select message sending `KeepAlive`"
                );

                peer_context.sender.send(Message::KeepAlive)?;
            }
        }
    }
}

/// If the client can download from remote Peer then select a [`Message`] to send to the remote
/// Peer.
async fn select_message(
    context: &Arc<Context>,
    peer_context: &Arc<PeerContext>,
) -> Result<Message> {
    log::debug!("\tselect_message:\tSelecting Message");

    loop {
        if let Ok(val) = peer_context.client_download() {
            if val {
                if let Ok(handler) = context.piece_handler.read() {
                    let block_info = handler.pick_piece().unwrap();

                    return Ok(Message::Request {
                        index: block_info.piece_index,
                        begin: block_info.begin,
                        length: BLOCK_SIZE,
                    });
                }
            }
        }
    }
}

/// Two way communication between peer handle and the torrent manager.
/// I/O events are handled by the [`PeerHandle`] itself but events like [`Message::Choke`] are
/// handled by the `Torrent' manager.
async fn handle_receiver(
    mut receiver: Receiver,
    context: Arc<Context>,
    peer_context: Arc<PeerContext>,
) -> Result<()> {
    use Message::*;

    loop {
        if let Some(msg) = receiver.recv().await {
            log::debug!("\thandle_receiver:\tReceived Message {}", msg);
            match msg {
                KeepAlive => {
                    if let Err(err) =
                        peer_context.set_connection_status(ConnectionStatus::Connected)
                    {
                        log::error!("Error:\t{}", err)
                    }
                }
                Choke => {
                    if let Err(err) = peer_context.set_peer_choking(ChokeStatus::Choked) {
                        log::error!("Error:\t{}", err)
                    }
                }
                Unchoke => {
                    if let Err(err) = peer_context.set_peer_choking(ChokeStatus::Unchoked) {
                        log::error!("Error:\t{}", err)
                    }
                }
                Interested => {
                    if let Err(err) = peer_context.set_peer_interested(Intrest::Interested) {
                        log::error!("Error:\t{}", err)
                    }
                }
                NotInterested => {
                    if let Err(err) = peer_context.set_peer_interested(Intrest::NotInterested) {
                        log::error!("Error:\t{}", err)
                    }
                }
                Have(index) => {
                    if let Err(err) = peer_context.set_index(index) {
                        log::error!("Error:\t{}", err)
                    }
                }
                Piece(block) => {
                    if let Err(err) = context.insert_block(block) {
                        log::error!("Error:\t{}", err)
                    }
                }
                Port(_) => {
                    log::info!("Port Implementation still pending")
                }
                Bitfield(bitfield) => match context.match_bitfield_len(bitfield.len()) {
                    Ok(res) => {
                        if res {
                            if let Err(err) = peer_context.set_bitfield(bitfield) {
                                log::error!("Error:\t{}", err)
                            }
                        } else {
                            let _ =
                                peer_context.set_connection_status(ConnectionStatus::Disconnected);
                            peer_context.close_session();
                            return Err(TorrusError::new("Bitfield length did not match"));
                        }
                    }
                    Err(err) => {
                        log::error!("Error:\t{}", err)
                    }
                },
                _ => {
                    unimplemented!("Implement all branches")
                }
            }
        }
    }
}
