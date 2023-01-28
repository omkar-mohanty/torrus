use tokio::task::JoinHandle;

use super::state::{ConnectionStatus, PeerState};
use crate::{Bitfield, PeerId, PieceIndex, Result, Sender, TorrusError};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Holds all information about the Peer.
/// Accessed by ['Torrent'] and [`PeerHandle`] in different threads.
pub struct PeerContext {
    /// Information about the connection state and [`Choked`]/[`Unchoked`] information
    peer_state: RwLock<PeerState>,
    /// 20 byte PeerId
    pub peer_id: PeerId,
    /// Channel to send messages to Peer
    pub sender: Sender,
    /// JoinHandle of [`PeerSession`]
    session_join_handle: JoinHandle<()>,
}

impl PeerContext {
    /// [`PeerContext`] should be constructed only when connection is active.
    pub fn new(
        peer_id: PeerId,
        sender: Sender,
        bitfield_len: usize,
        session_join_handle: JoinHandle<()>,
    ) -> Self {
        let peer_state = RwLock::new(PeerState::new(bitfield_len));
        Self {
            peer_state,
            peer_id,
            sender,
            session_join_handle,
        }
    }

    /// Get write lock to [`PeerState`]
    fn get_state_write(&self) -> Result<RwLockWriteGuard<PeerState>> {
        let peer_state = match self.peer_state.write() {
            Ok(state) => state,
            Err(err) => {
                log::error!("Error:\t{}", err);
                return Err(TorrusError::new(&err.to_string()));
            }
        };
        Ok(peer_state)
    }

    /// Get reader lock to [`PeerState`]
    fn get_state_read(&self) -> Result<RwLockReadGuard<PeerState>> {
        let peer_state = match self.peer_state.read() {
            Ok(state) => state,
            Err(err) => {
                log::error!("Error:\t{}", err);
                return Err(TorrusError::new(&err.to_string()));
            }
        };
        Ok(peer_state)
    }

    pub fn set_peer_choking(&self, choking: bool) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.set_peer_choking(choking);

        Ok(())
    }

    pub fn set_peer_interested(&self, interested: bool) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.set_peer_interested(interested);

        Ok(())
    }

    pub fn set_bitfield(&self, bitfield: Bitfield) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.set_bitfield(bitfield);

        Ok(())
    }

    pub fn set_index(&self, index: PieceIndex) -> Result<()> {
        let mut state = self.get_state_write()?;

        if index >= state.bitfield.len() {
            let msg = format!("Cannot set index {} for bitfield of length {}", index, state.bitfield.len());
            return Err(TorrusError::new(&msg));
        }

        state.set_index(index);

        Ok(())
    }

    pub fn set_connection_status(&self, connection_status: ConnectionStatus) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.set_connection_status(connection_status);

        Ok(())
    }

    pub fn close_session(&self) {
        self.session_join_handle.abort();
    }
}
