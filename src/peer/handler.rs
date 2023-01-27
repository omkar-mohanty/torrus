use crate::error::TorrusError;
use crate::message::Message;
use crate::peer::state::ConnectionStatus;
use crate::peer::PeerSession;
use crate::torrent::Context;
use crate::{PeerId, Receiver, Sender};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::{net::TcpStream, sync::mpsc::unbounded_channel};

use super::state::PeerState;

/// Struct to send events across tasks
pub struct PeerEvent {
    pub peer_id: PeerId,
    pub peer_state: PeerState,
}

type Result<T> = std::result::Result<T, crate::error::TorrusError>;

/// The torrent manager handles peer through this struct.
/// Communication mostly takes place by sending messages across a channel
pub struct PeerHandle {
    /// Connection state of the peer
    pub peer_state: PeerState,
    /// Command sender for the peer
    pub sender: Sender,
    /// Receiver for events from the peer
    join_handle: JoinHandle<Result<()>>,
}

impl PeerHandle {
    /// Immediately start the session while the handle is being constructed
    pub fn new(stream: TcpStream, context: Arc<Context>, peer_id: PeerId) -> Self {
        let peer_state = PeerState::new();

        let (msg_send, receiver) = unbounded_channel();

        let (sender, mut peer_session) = PeerSession::new(msg_send);

        tokio::spawn(async move {
            if let Err(_) = peer_session.start(stream).await {
                return;
            }
        });

        let join_handle = tokio::spawn(async move {
            handle_receiver(receiver, context, peer_id).await?;
            Ok::<(), crate::error::TorrusError>(())
        });

        Self {
            peer_state,
            sender,
            join_handle,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.join_handle.is_finished()
    }

    pub fn set_state(&mut self, peer_state: PeerState) {
        self.peer_state = peer_state;
    }
}

/// Two way communication between peer handle and the torrent manager.
/// I/O events are handled by the [`PeerHandle`] itself but events like [`Message::Choke`] are
/// handled by the `Torrent' manager.
async fn handle_receiver(
    mut receiver: Receiver,
    context: Arc<Context>,
    peer_id: PeerId,
) -> Result<()> {
    use Message::*;

    loop {
        if let Some(msg) = receiver.recv().await {
            match msg {
                KeepAlive => {
                    let mut peer_state = PeerState::new();

                    peer_state.connection_status = ConnectionStatus::Connected;

                    let message = PeerEvent {
                        peer_id,
                        peer_state,
                    };

                    if let Err(err) = context.sender.send(message) {
                        log::error!("Error {}", err);
                        return Ok(());
                    }
                }
                Choke => {}
                Piece(block) => {
                    if context.piece_handler.try_write().is_ok() {
                        let piece_handler = &mut context.piece_handler.write().unwrap();

                        if let Err(err) = piece_handler.insert_block(block) {
                            return Err(TorrusError::new(&err.to_string()));
                        }
                    }
                }
                _ => {
                    unimplemented!("Implement all branches")
                }
            }
        }
    }
}
