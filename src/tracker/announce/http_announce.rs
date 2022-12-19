use super::{AnnounceRequest, AnnounceResponse};
use hyper::body::HttpBody;
use hyper::{Body, Client, Request, Response, StatusCode};
use hyper_tls::HttpsConnector;
use url::form_urlencoded::byte_serialize;
use url::Url;

struct QueryBuilder {
    query: String,
}

impl QueryBuilder {
    fn new() -> Self {
        Self {
            query: String::new(),
        }
    }

    fn append_pair(mut self, key: &str, value: &str) -> Self {
        let pair = format!("{}={}", key, value);
        if self.query.is_empty() {
            self.query += pair.as_str();
            self
        } else {
            self.query = self.query + "&" + pair.as_str();
            self
        }
    }

    fn build(self) -> String {
        self.query
    }
}

fn build_https_client() -> Client<HttpsConnector<hyper::client::HttpConnector>> {
    let client = Client::builder();
    client.build::<_, Body>(HttpsConnector::new())
}

fn build_query(request: AnnounceRequest) -> String {
    // Serealize info_hash to percent encoding
    let info_hash_str: String = byte_serialize(&request.info_hash).collect();

    // Serealize peer_id to percent encoding
    let peer_id_str: String = byte_serialize(&request.peer_id).collect();

    //Build GET request query
    QueryBuilder::new()
        .append_pair("info_hash", &info_hash_str)
        .append_pair("peer_id", &peer_id_str)
        .append_pair("downloaded", &request.downloaded.to_string())
        .append_pair("left", &request.left.to_string())
        .append_pair("uploaded", &request.uploaded.to_string())
        .append_pair("event", &request.event)
        .append_pair("ip_address", &request.ip_address.to_string())
        .append_pair("key", &request.key.to_string())
        .append_pair("num_want", &request.num_want.to_string())
        .append_pair("port", &request.port.to_string())
        .append_pair("no_peer_id", "0")
        .append_pair("compact", "1")
        .build()
}

fn build_announce_request(
    request: AnnounceRequest,
    mut url: Url,
) -> Result<Request<Body>, Box<dyn std::error::Error>> {
    let query = build_query(request);

    url.set_query(Some(&query));

    // Parse the url as hyper Uri
    let uri = url.as_str().parse::<hyper::Uri>()?;

    // Construct the HTTP request object
    let req = hyper::Request::get(uri).body(Body::empty())?;

    Ok(req)
}

async fn send_request(
    request: Request<Body>,
    url: Url,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
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

pub async fn announce(
    request: AnnounceRequest,
    url: Url,
) -> Result<AnnounceResponse, Box<dyn std::error::Error>> {
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
    println!("{}", announce_response);
    Ok(announce_response)
}
