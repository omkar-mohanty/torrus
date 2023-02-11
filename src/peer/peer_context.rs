use super::state::{ConnectionStatus, Intrest, State};
use crate::peer::state::ChokeStatus;
use crate::{Bitfield, PeerId, PieceIndex, Result, Sender, TorrusError};
use std::sync::{Mutex, MutexGuard};
use tokio::task::JoinHandle;

/// Holds all information about the Peer.
/// Accessed by ['Torrent'] and [`PeerHandle`] in different threads.
pub struct PeerContext {
    /// Information about the connection state and [`Choked`]/[`Unchoked`] information
    state: Mutex<State>,
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
        let state = Mutex::new(State::new(bitfield_len));
        Self {
            state,
            peer_id,
            sender,
            session_join_handle,
        }
    }

    pub fn peer_interested(&self) -> Intrest {
        self.get_mutex(|state| state.peer_state.intrest)
    }

    pub fn peer_chocking(&self) -> ChokeStatus {
        self.get_mutex(|state| state.peer_state.choke)
    }

    pub fn client_interested(&self) -> Intrest {
        self.get_mutex(|state| state.client_state.intrest)
    }

    pub fn client_choked(&self) -> ChokeStatus {
        self.get_mutex(|state| state.client_state.choke)
    }

    /// Get write lock to [`PeerState`]
    pub fn get_mutex<F, T>(&self, func: F) -> T
    where
        F: FnOnce(MutexGuard<State>) -> T,
    {
        let state = self.state.lock().unwrap();
        func(state)
    }

    pub fn set_peer_choking(&self, choking: ChokeStatus) {
        self.get_mutex(|mut state| {
            state.peer_state.choke = choking;
        })
    }

    pub fn set_peer_interested(&self, interested: Intrest) {
        self.get_mutex(|mut state| {
            state.peer_state.intrest = interested;
        })
    }

    pub fn set_bitfield(&self, bitfield: Bitfield) {
        self.get_mutex(|mut state| state.set_bitfield(bitfield))
    }

    pub fn set_index(&self, index: PieceIndex) -> Result<()> {
        self.get_mutex(|mut state| {
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
        })
    }

    pub fn set_connection_status(&self, connection_status: ConnectionStatus) {
        self.get_mutex(|mut state| {
            state.peer_state.connection_status = connection_status;
        })
    }

    pub fn close_session(&self) {
        self.session_join_handle.abort();
    }

    pub fn client_download(&self) -> bool {
        self.get_mutex(|state| {
            if let (ChokeStatus::Unchoked, Intrest::Interested) =
                (state.peer_state.choke, state.client_state.intrest)
            {
                true
            } else {
                false
            }
        })
    }

    pub fn set_client_interest(&self, interest: Intrest) {
        self.get_mutex(|mut state| {
            state.client_state.intrest = interest;
        })
    }
}
