use url::Url;

use crate::request::{TrackerRequest, TrackerRequestBuilder};

use super::{
    http::HttpTracker,
    udp::{UdpRequestBuilder, UdpTracker},
};

pub struct Tracker {
    tracker_type: Type,
}

impl Tracker {
    pub fn new(url: Url) -> Self {
        use Type::*;
        let tracker_type = match url.scheme() {
            "https" | "http" => Http(HttpTracker::new(url)),
            "udp" => Udp(UdpTracker::new(url)),
            _ => panic!("Magnet Links not supported yet!"),
        };

        Self { tracker_type }
    }

    pub fn request_builder(&self) -> impl TrackerRequestBuilder {
        use Type::*;
        match &self.tracker_type {
            Http(_) => todo!(),
            Udp(_) => UdpRequestBuilder,
        }
    }

    pub fn send_request(&self, request: impl TrackerRequest) {
        use Type::*;
        match &self.tracker_type {
            Http(_) => todo!(),
            Udp(tracker) => tracker.send_request(request),
        };
    }
}

enum Type {
    Http(HttpTracker),
    Udp(UdpTracker),
}
