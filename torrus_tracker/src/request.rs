use torrus_core::id::ID;

#[derive(Default)]
pub struct TrackerRequest {
    pub(crate) info_hash: ID,
    pub(crate) peer_id: ID,
    pub(crate) port: u16,
    pub(crate) uploaded: u64,
    pub(crate) downloaded: u64,
    pub(crate) left: u64,
    pub(crate) event: String,
    pub(crate) ip_address: u32,
    pub(crate) key: u32,
    pub(crate) num_want: i32,
}

impl TrackerRequest {
    pub fn builder() -> Self {
        Self::default()
    }

    pub fn info_hash(mut self, id: ID) -> Self {
        self.info_hash = id;
        self
    }

    pub fn set_peer_id(mut self, id: ID) -> Self {
        self.peer_id = id;
        self
    }

    pub fn set_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn set_downloaded(mut self, downloaded: u64) -> Self {
        self.downloaded = downloaded;
        self
    }

    pub fn set_event(mut self, left: u64) -> Self {
        self.left = left;
        self
    }

    pub fn set_ip(mut self, ip: u32) -> Self {
        self.ip_address = ip;
        self
    }

    pub fn set_key(mut self, key: u32) -> Self {
        self.key = key;
        self
    }

    pub fn set_num_want(mut self, num_want: i32) -> Self {
        self.num_want = num_want;
        self
    }
}
