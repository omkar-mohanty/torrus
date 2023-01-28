use crate::{Bitfield, PieceIndex};

pub struct PeerState {
    /// Remote Peer chocking
    pub peer_choking: bool,
    /// Remote Peer interested
    pub peer_interested: bool,
    /// Client chocking
    pub client_choking: bool,
    /// Client interested
    pub client_intrested: bool,
    /// Remote Peer connection status
    pub connection_status: ConnectionStatus,
    /// Bitfield of the remote Peer
    pub bitfield: Bitfield,
}

pub enum ConnectionStatus {
    Connected,
    Unkown,
}

impl PeerState {
    pub fn new(bitfield_len: usize) -> Self {
        let bitfield = Bitfield::with_capacity(bitfield_len);
        Self {
            peer_choking: true,
            peer_interested: false,
            client_choking: true,
            client_intrested: true,
            connection_status: ConnectionStatus::Unkown,
            bitfield,
        }
    }

    pub fn set_peer_choking(&mut self, choking: bool) {
        self.peer_choking = choking;
    }

    pub fn set_peer_interested(&mut self, interested: bool) {
        self.peer_interested = interested;
    }

    pub fn set_client_choking(&mut self, choking: bool) {
        self.client_choking = choking;
    }

    pub fn set_client_interested(&mut self, interested: bool) {
        self.client_intrested = interested;
    }

    pub fn set_connection_status(&mut self, connection_status: ConnectionStatus) {
        self.connection_status = connection_status;
    }

    pub fn set_bitfield(&mut self, bitfield: Bitfield) {
        self.bitfield = bitfield;
    }

    pub fn set_index(&mut self, index: PieceIndex) {
        self.bitfield.set(index, true);
    }

    pub fn unset_index(&mut self, index: PieceIndex) {
        self.bitfield.set(index, false);
    }
}
