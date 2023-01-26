/// A pper can be alive, dead or pending
pub struct PeerState {
    pub peer_choking: bool,
    pub peer_interested: bool,
    pub am_choking: bool,
    pub am_intrested: bool,
}

impl PeerState {
    pub fn new() -> Self {
        Self {
            peer_choking: true,
            peer_interested: false,
            am_choking: true,
            am_intrested: true,
        }
    }
}
