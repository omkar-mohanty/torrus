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

    pub fn info_hash(mut req: Self, id: ID) -> Self {
        req.info_hash = id;
        req
    }

    pub fn set_peer_id(mut req: Self, id: ID) -> Self {
        req.peer_id = id;
        req
    }

    pub fn set_port(mut req: Self, port: u16) -> Self {
        req.port = port;
        req
    }

    pub fn set_downloaded(mut req: Self, downloaded: u64) -> Self {
        req.downloaded = downloaded;
        req
    }

    pub fn set_event(mut req: Self, left: u64) -> Self {
        req.left = left;
        req
    }

    pub fn set_ip(mut req: Self, ip: u32) -> Self {
        req.ip_address = ip;
        req
    }

    pub fn set_key(mut req: Self, key: u32) -> Self {
        req.key = key;
        req
    }

    pub fn set_num_want(mut req: Self, num_want: i32) -> Self {
        req.num_want = num_want;
        req
    }
}
