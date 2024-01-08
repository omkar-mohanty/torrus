use std::str::FromStr;

use reqwest::Client;
use url::Url;
use uuid::Version;

struct HttpTracker {
    url: Url,
    client: Client,
}

struct HttpQuery {
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
    port: u8,
    uploaded: u8,
    downloaded: u8,
    left: u8,
    compact: u8,
    no_peer_id: u8,
    event: u8,
    ip: Option<u8>,
    numwant: Option<u8>,
    key: Option<u8>,
    trackerid: Option<u8>,
}

impl HttpTracker {
    pub fn new(url: Url) -> Self {
        let client = Client::new();
        HttpTracker { url, client }
    }

    fn send_request(&mut self, http_query: HttpQuery) {
        let mut query = Vec::new();
        let request = self.client.get(self.url.as_str());
        request.query(&query);
    }
}

struct UdpTracker {
    url: Url,
}

impl UdpTracker {
    pub fn new(url: Url) -> Self {
        UdpTracker { url }
    }
}

enum TrackerType {
    Http(HttpTracker),
    Udp(UdpTracker),
}

impl TrackerType {
    pub fn new(url: String) -> Self {
        let url = Url::from_str(&url).unwrap();

        match url.scheme() {
            "http" | "https" => Self::Http(HttpTracker::new(url)),
            "udp" => Self::Udp(UdpTracker::new(url)),
            _ => todo!(),
        }
    }
}

pub struct Tracker {
    tracker_type: TrackerType,
}
