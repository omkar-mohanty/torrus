use super::state::{ConnectionStatus, Intrest, State};
use crate::peer::state::ChokeStatus;
use crate::{Bitfield, PeerId, PieceIndex, Result, Sender, TorrusError};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinHandle;

/// Holds all information about the Peer.
/// Accessed by ['Torrent'] and [`PeerHandle`] in different threads.
pub struct PeerContext {
    /// Information about the connection state and [`Choked`]/[`Unchoked`] information
    state: RwLock<State>,
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
        let state = RwLock::new(State::new(bitfield_len));
        Self {
            state,
            peer_id,
            sender,
            session_join_handle,
        }
    }

    pub fn peer_interested(&self) -> Result<Intrest> {
        let state = self.get_state_read()?;

        Ok(state.peer_state.intrest)
    }

    pub fn peer_chocking(&self) -> Result<ChokeStatus> {
        let state = self.get_state_read()?;

        Ok(state.peer_state.choke)
    }

    pub fn client_interested(&self) -> Result<Intrest> {
        let state = self.get_state_read()?;

        Ok(state.client_state.intrest)
    }

    pub fn client_choked(&self) -> Result<ChokeStatus> {
        let state = self.get_state_read()?;

        Ok(state.client_state.choke)
    }

    /// Get write lock to [`PeerState`]
    fn get_state_write(&self) -> Result<RwLockWriteGuard<State>> {
        let peer_state = match self.state.write() {
            Ok(state) => state,
            Err(err) => {
                log::error!("Error:\t{}", err);
                return Err(TorrusError::new(&err.to_string()));
            }
        };
        Ok(peer_state)
    }

    /// Get reader lock to [`PeerState`]
    fn get_state_read(&self) -> Result<RwLockReadGuard<State>> {
        let peer_state = match self.state.read() {
            Ok(state) => state,
            Err(err) => {
                log::error!("Error:\t{}", err);
                return Err(TorrusError::new(&err.to_string()));
            }
        };
        Ok(peer_state)
    }

    pub fn set_peer_choking(&self, choking: ChokeStatus) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.peer_state.choke = choking;

        Ok(())
    }

    pub fn set_peer_interested(&self, interested: Intrest) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.peer_state.intrest = interested;

        Ok(())
    }

    pub fn set_bitfield(&self, bitfield: Bitfield) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.set_bitfield(bitfield);

        Ok(())
    }

    pub fn set_index(&self, index: PieceIndex) -> Result<()> {
        let mut state = self.get_state_write()?;

        if index >= state.peer_state.bitfield.len() {
            let msg = format!(
                "Cannot set index {} for bitfield of length {}",
                index,
                state.peer_state.bitfield.len()
            );
            return Err(TorrusError::new(&msg));
        }

        state.set_index(index);

        Ok(())
    }

    pub fn set_connection_status(&self, connection_status: ConnectionStatus) -> Result<()> {
        let mut state = self.get_state_write()?;

        state.peer_state.connection_status = connection_status;

        Ok(())
    }

    pub fn close_session(&self) {
        self.session_join_handle.abort();
    }

    pub fn client_download(&self) -> Result<bool> {
        let state = self.get_state_read()?;

        if let (ChokeStatus::Unchoked, Intrest::Interested) =
            (state.peer_state.choke, state.client_state.intrest)
        {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
