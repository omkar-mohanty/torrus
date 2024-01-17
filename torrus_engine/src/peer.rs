use torrus_core::prelude::{ChokeStatus, IntrestStatus, PeerInfo, PeerState, Sha1Hash, ID};

pub struct Peer {
    pub(crate) peer_info: PeerInfo,
    pub(crate) state: PeerState,
}

impl Peer {
    pub fn new(peer_info: PeerInfo) -> Self {
        Peer {
            peer_info,
            state: PeerState::default(),
        }
    }
}

impl Sha1Hash for Peer {
    fn as_sha1(&self) -> ID {
        self.peer_info.id
    }
}
