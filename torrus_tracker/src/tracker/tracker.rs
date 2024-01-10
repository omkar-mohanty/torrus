use std::str::FromStr;

use super::{http::HttpTracker, udp::UdpTracker};
use crate::request::TrackerRequest;
use anyhow::{Ok, Result};
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

    pub async fn send_request(&mut self, request: TrackerRequest) -> Result<()> {
        use Type::*;

        match &mut self.tracker_type {
            Http(ref mut tracker) => tracker.send_request(request).await?,
            Udp(tracker) => tracker.send_request(request).await?,
        };

        Ok(())
    }
}

enum Type {
    Http(HttpTracker),
    Udp(UdpTracker),
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::fs;
    use torrus_core::prelude::*;

    #[tokio::test]
    async fn test_tracker() -> Result<()> {
        let dir = fs::read("../resources/ubuntu-22.10-desktop-amd64.iso.torrent").unwrap();
        let metainfo = Metainfo::new(&dir).unwrap();

        if let Some(al) = metainfo.announce_list {
            for a in al {
                let mut tracker = Tracker::new(&a[0]);
                tracker.send_request(TrackerRequest::default()).await?;
            }
        }

        Ok(())
    }
}
