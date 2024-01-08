use crate::{TrackerRequest, TrackerRequestBuilder};
use url::Url;

pub struct UdpRequest;

pub struct UdpRequestBuilder;

impl TrackerRequest for UdpRequest {}

impl TrackerRequestBuilder for UdpRequestBuilder {
    fn build(self) -> impl TrackerRequest {
        UdpRequest
    }
}

pub struct UdpTracker {
    url: Url,
}

impl UdpTracker {
    pub fn new(url: Url) -> Self {
        Self { url }
    }

    pub fn send_request(&self, _udp_request: impl TrackerRequest) {
        todo!()
    }
}
