use reqwest::Client;
use url::Url;

pub struct HttpTracker {
    url: Url,
    client: Client,
}

impl HttpTracker {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            client: Client::new(),
        }
    }
}
