use super::peer_context::PeerContext;
use crate::error::TorrusError;
use crate::message::Message;
use crate::peer::session::PeerSession;
use crate::peer::state::{ChokeStatus, ConnectionStatus, Intrest};
use crate::torrent::Context;
use crate::{PeerId, Receiver};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::{net::TcpStream, sync::mpsc::unbounded_channel};

pub fn new_peer(stream: TcpStream, context: Arc<Context>, peer_id: PeerId) -> PeerHandle {
    PeerHandle::new(stream, context, peer_id)
}

type Result<T> = std::result::Result<T, crate::error::TorrusError>;

/// The torrent manager handles peer through this struct.
/// Communication mostly takes place by sending messages across a channel
pub struct PeerHandle {
    /// Context shared by [`Torrent`] and [`PeerHandle`]
    pub peer_context: Arc<PeerContext>,
    /// Sender and receiver join handle
    pub join_handle: JoinHandle<()>,
    /// Time of the last send message
    pub time_last_msg: Option<Instant>,
    /// Context of the torrent
    pub torrent_context: Arc<Context>,
}

impl PeerHandle {
    /// Immediately start the session while the handle is being constructed
    pub fn new(stream: TcpStream, torrent_context: Arc<Context>, peer_id: PeerId) -> Self {
        let (msg_send, receiver) = unbounded_channel();

        let (sender, mut peer_session) = PeerSession::new(msg_send);

        let bitfield_len = torrent_context.metainfo.total_pieces();

        let peer_session_handle = tokio::spawn(async move {
            if (peer_session.start(stream).await).is_err() {
                return;
            }
        });

        let peer_context = PeerContext::new(peer_id, sender, bitfield_len, peer_session_handle);

        let peer_context = Arc::new(peer_context);
        let task_peer_context = Arc::clone(&peer_context);

        let receiver_context = Arc::clone(&torrent_context);

        let receiver_join_handle =
            handle_receiver(receiver, receiver_context, Arc::clone(&task_peer_context));

        let join_handle = tokio::spawn(async move {
            if let Err(err) = receiver_join_handle.await {
                log::error!("\tjoin_handle_thread:\t{}", err);
            }
        });

        Self {
            peer_context,
            join_handle,
            time_last_msg: None,
            torrent_context,
        }
    }

    /// Send messages to the remote Peer
    pub fn send(&mut self, msg: Message) -> Result<()> {
        match self.peer_context.sender.send(msg) {
            Ok(()) => {
                self.time_last_msg = Some(Instant::now());
                Ok(())
            }
            Err(err) => Err(TorrusError::new(&err.to_string())),
        }
    }

    pub fn select_message(&self) -> Option<Message> {
        use Message::*;

        let dur = Instant::now() - self.time_last_msg.unwrap_or(Instant::now());

        let peer_bitfield = self
            .peer_context
            .get_mutex(|context| context.peer_state.bitfield.clone());

        let msg = self.torrent_context.get_mutex(|context| {
            for (index, val) in peer_bitfield.iter().enumerate() {
                if *val {
                    let block_info = context.pick_piece(&index);

                    return Some(Request(block_info));
                }
            }
            None
        });

        match msg {
            Some(msg) => Some(msg),
            None => {
                if dur >= Duration::from_secs(120) {
                    Some(KeepAlive)
                } else {
                    Some(Interested)
                }
            }
        }
    }

    /// Close all communications with the Peer
    pub fn close(self) {
        self.peer_context.close_session();
        self.join_handle.abort();
    }

    pub async fn check_connection(&self) -> bool {
        self.join_handle.is_finished()
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
                    peer_context.set_connection_status(ConnectionStatus::Connected);
                }
                Choke => peer_context.set_peer_choking(ChokeStatus::Choked),
                Unchoke => peer_context.set_peer_choking(ChokeStatus::Unchoked),
                Interested => peer_context.set_peer_interested(Intrest::Interested),
                NotInterested => peer_context.set_peer_interested(Intrest::NotInterested),
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
                    true => {
                        log::debug!(
                            "\thandle_receiver:\thave : {}, not : {}",
                            bitfield.count_ones(),
                            bitfield.count_zeros()
                        );
                        peer_context.set_bitfield(bitfield);
                    }
                    false => {
                        let _ = peer_context.set_connection_status(ConnectionStatus::Disconnected);
                        peer_context.close_session();
                        return Err(TorrusError::new(&format!(
                            "Got Bitfield of length {}",
                            bitfield.len()
                        )));
                    }
                },
                _ => {
                    unimplemented!("Implement all branches")
                }
            }
        }
    }
}
