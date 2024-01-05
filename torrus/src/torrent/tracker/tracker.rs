use url::Url;

struct HttpTracker {
    url: Url,
}

struct UdpTracker {
    url: Url,
}

pub enum Tracker {
    Http(HttpTracker),
    Udp(UdpTracker),
}

impl Tracker {
    pub fn new(_url: String) -> Self {
        todo!()
    }
}
