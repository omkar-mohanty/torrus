use crate::TrackerRequest;
use anyhow::Result;
use url::Url;

pub struct UdpTracker {
    url: Url,
}

impl UdpTracker {
    pub fn new(url: Url) -> Self {
        Self { url }
    }

    pub async fn send_request(&self, _udp_request: TrackerRequest) -> Result<()> {
        todo!()
    }
}
