use super::{http::HttpTracker, udp::UdpTracker};
use crate::{TrackerRequest, TrackerResponse};
use anyhow::Result;
use std::str::FromStr;
use torrus_core::id::ID;
use url::Url;

pub struct Tracker {
    tracker_type: Type,
}

impl Tracker {
    pub fn new(url: &str) -> Self {
        let url = Url::from_str(url).unwrap();
        use Type::*;
        let tracker_type = match url.scheme() {
            "https" | "http" => Http(HttpTracker::new(url)),
            "udp" => Udp(UdpTracker::new(url)),
            _ => panic!("Magnet Links not supported yet!"),
        };

        Self { tracker_type }
    }

    pub async fn send_request(&mut self, request: TrackerRequest) -> Result<TrackerResponse> {
        use Type::*;

        match &mut self.tracker_type {
            Http(ref mut tracker) => tracker.send_request(request).await,
            Udp(ref mut tracker) => tracker.send_request(request).await,
        }
    }

    pub async fn announce(&mut self, id: ID) -> Result<TrackerResponse> {
        let tracker_request = TrackerRequest::builder().info_hash(id).set_port(6881);
        self.send_request(tracker_request).await
    }
}

enum Type {
    Http(HttpTracker),
    Udp(UdpTracker),
}
