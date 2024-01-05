use std::str::FromStr;

use url::Url;

struct HttpTracker {
    url: Url,
}

impl HttpTracker {
    pub fn new(url: Url) -> Self {
        HttpTracker { url}
    }
}

struct UdpTracker {
    url: Url,
}

impl UdpTracker {
    pub fn new(url : Url) -> Self {
        UdpTracker {
            url
        }
    }
}

pub enum Tracker {
    Http(HttpTracker),
    Udp(UdpTracker),
}

impl Tracker {
    pub fn new(url: String) -> Self {
        let url = Url::from_str(&url).unwrap();

        match url.scheme() {
            "http" | "https" => {
                Self::Http(HttpTracker::new(url))
            },
            "udp" => {
                Self::Udp(UdpTracker::new(url))
            }
            _ => todo!()
        }
    }
}
