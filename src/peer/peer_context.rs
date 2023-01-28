use super::state::{ConnectionStatus, PeerState};
use crate::{Bitfield, PeerId, PieceIndex, Result, Sender, TorrusError};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Holds all information about the Peer.
/// Accessed by ['Torrent'] and [`PeerHandle`] in different threads.
pub struct PeerContext {
    /// Information about the connection state and [`Choked`]/[`Unchoked`] information
    pub peer_state: RwLock<PeerState>,
    /// 20 byte PeerId
    pub peer_id: PeerId,
    /// Channel to send messages to Peer
    pub sender: Sender,
}

impl PeerContext {
    /// [`PeerContext`] should be constructed only when connection is active.
    pub fn new(peer_id: PeerId, sender: Sender) -> Self {
        let peer_state = RwLock::new(PeerState::new());
        Self {
            peer_state,
            peer_id,
            sender,
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

        state.set_index(index);

        Ok(())
    }

    pub fn set_connection_status(&self, connection_status: ConnectionStatus) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.set_connection_status(connection_status);

        Ok(())
    }
}
