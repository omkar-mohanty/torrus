use crate::{Bitfield, PieceIndex};

pub struct State {
    /// The current state the Peer has with the client
    pub peer_state: PeerState,
    /// THe current state the client has with the peer
    pub client_state: ClientState,
}

impl State {
    pub fn new(bitfield_len: usize) -> Self {
        Self {
            peer_state: PeerState::new(bitfield_len),
            client_state: ClientState::new(),
        }
    }

    pub fn set_bitfield(&mut self, bitfield: Bitfield) {
        self.peer_state.bitfield = bitfield;
    }

    pub fn set_index(&mut self, index: usize) {
        self.peer_state.set_index(index)
    }
}

pub struct PeerState {
    /// THe Peer can be intresetd for downloading from client or not interested.
    pub intrest: Intrest,
    /// The peer has [`Choke::Choked`] or [`Choke:: Unchoked`] the client
    pub choke: ChokeStatus,
    /// Remote Peer connection status
    pub connection_status: ConnectionStatus,
    /// Bitfield of the remote Peer
    pub bitfield: Bitfield,
}

pub struct ClientState {
    /// The client has [`Choke::Choked`] or [`Choke:: Unchoked`] the client
    pub choke: ChokeStatus,
    /// Client interested or not interested
    pub intrest: Intrest,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            choke: ChokeStatus::Choked,
            intrest: Intrest::NotInterested,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ChokeStatus {
    Choked,
    Unchoked,
}

#[derive(Clone, Copy)]
pub enum Intrest {
    Interested,
    NotInterested,
}

#[derive(Clone, Copy)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

impl PeerState {
    pub fn new(bitfield_len: usize) -> Self {
        let bitfield = Bitfield::with_capacity(bitfield_len);
        Self {
            intrest: Intrest::NotInterested,
            choke: ChokeStatus::Choked,
            connection_status: ConnectionStatus::Connected,
            bitfield,
        }
    }

    pub fn set_index(&mut self, index: PieceIndex) {
        self.bitfield.set(index, true);
    }
}
