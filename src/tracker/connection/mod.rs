use super::error::TrackerError;
use super::{TrackerRequest, TrackerResponse};
use hyper::body::HttpBody;
use hyper::{client::connect::Connect, Body, Client, Request, Response, StatusCode};
use hyper_tls::HttpsConnector;
use url::Url;

type Bytes = Vec<u8>;
type Result<T> = std::result::Result<T, TrackerError>;

pub fn from_url<T: ConnectionMessage>(url: Url) -> Box<dyn Session<T>> {
    from_url_udp(url)
}

fn from_url_udp<T: ConnectionMessage>(url: Url) -> Box<dyn Session<T>> {
    Box::new(UdpSession { url })
}

fn from_url_http<T: ConnectionMessage<MessageType = String>>(url: Url) -> Box<dyn Session<T>> {
    match url.scheme() {
        "https" => Box::new(HttpSession {
            url,
            client: build_https_client(),
        }),
        "http" => Box::new(HttpSession {
            url,
            client: Client::new(),
        }),
        _ => todo!(),
    }
}

pub trait Session<T: ConnectionMessage> {
    fn send(&self, message: T) -> Result<Bytes>;
}

struct UdpSession {
    url: Url,
}

impl<T: ConnectionMessage> Session<T> for UdpSession {
    fn send(&self, message: T) -> Result<Bytes> {
        todo!()
    }
}

struct HttpSession<T> {
    url: Url,
    client: Client<T>,
}

impl<T> HttpSession<T>
where
    T: Connect + Clone + Send + Sync + 'static,
{
    async fn send_message(
        &self,
        message: impl ConnectionMessage<MessageType = String>,
    ) -> Result<Bytes> {
        let query = message.serealize();

        let mut url = self.url.clone();

        url.set_query(Some(&query));

        let req = hyper::Request::get(url.as_str().parse::<hyper::Uri>()?).body(Body::empty())?;

        let res = self.client.request(req);

        let body = res.await.unwrap().into_body().data().await.unwrap()?;

        Ok(Vec::from(body))
    }
}

impl<T, K> Session<T> for HttpSession<K>
where
    T: ConnectionMessage<MessageType = String>,
    K: Connect + Clone + Send + Sync + 'static,
{
    fn send(&self, message: T) -> Result<Bytes> {
        self.send_message(message);
        todo!()
    }
}

pub trait ConnectionMessage<MessageType = Bytes> {
    type MessageType;

    fn serealize(self) -> Self::MessageType;
}

fn build_https_client() -> Client<HttpsConnector<hyper::client::HttpConnector>> {
    let client = Client::builder();
    client.build::<_, Body>(HttpsConnector::new())
}

/// Build TrackerRequest object
fn build_announce_request(request: TrackerRequest, mut url: Url) -> Result<Request<Body>> {
    // Build query string
    let query = request.serealize();

    url.set_query(Some(&query));

    // Parse the url as hyper Uri
    let uri = url.as_str().parse::<hyper::Uri>()?;

    // Construct the HTTP request object
    let req = hyper::Request::get(uri).body(Body::empty())?;

    Ok(req)
}

/// Construct client depending on with or without tls
async fn send_request(request: Request<Body>, url: Url) -> Result<Response<Body>> {
    let res = match url.scheme() {
        "http" => {
            let client = Client::new();
            client.request(request).await?
        }
        "https" => {
            let client = build_https_client();
            client.request(request).await?
        }
        _ => {
            panic!("Invalid scheme");
        }
    };

    Ok(res)
}

/// Implements the announce protocol from Bittorrent specification
///
/// Note this implementation infinitely redirects
pub async fn announce(request: TrackerRequest, url: Url) -> Result<TrackerResponse> {
    // Build http announce request
    let req = build_announce_request(request.clone(), url.clone())?;

    // Send the request to the tracker
    let mut res = send_request(req, url.clone()).await?;

    while res.status() == StatusCode::TEMPORARY_REDIRECT {
        // Get the URL from the "Location" header
        let location = res.headers().get("Location").unwrap();
        let location_url = location.to_str().unwrap();
        let url = url.clone();

        // Construct a new request to the URL
        let redirect_url = url.join(location_url)?;

        let new_req = build_announce_request(request.clone(), url.clone())?;

        // Send the new request
        res = send_request(new_req, redirect_url).await?;
    }

    let data_bytes = res.into_body().data().await.unwrap()?;

    let slice = &data_bytes[..];

    let announce_response = serde_bencode::from_bytes(slice)?;
    Ok(announce_response)
}

#[cfg(test)]
mod tests {}
