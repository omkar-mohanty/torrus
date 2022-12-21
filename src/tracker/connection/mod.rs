use async_trait::async_trait;
use error::ConnectionError;
use hyper::body::HttpBody;
use hyper::{client::connect::Connect, Body, Client};
use hyper::{Response, StatusCode};
use hyper_tls::HttpsConnector;
use std::collections::HashMap;
use std::ops::Deref;
use url::Url;

mod error;

type Result<T> = std::result::Result<T, ConnectionError>;

pub struct Query(HashMap<String, String>);
pub struct Bytes(Vec<u8>);

impl Query {
    pub fn new(map: HashMap<String, String>) -> Self {
        Self(map)
    }
}

impl Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl Into<String> for Query {
    fn into(self) -> String {
        let map = self.0;
        let mut res = String::new();
       
        for (key,value) in map.iter() {
            if res.is_empty() {
                res+=key;
                res+="=";
                res+=value;
            }

            res+="&";
            res+=key;
            res+="=";
            res+=value;
        }
        println!("{}",res);
        res
    }
}

pub fn from_url<T: Into<Query> + Send + 'static>(url: Url) -> Box<dyn Session<T>> {
    match url.scheme() {
        "http" | "https" => from_url_http(url),
        "udp" => from_url_udp(url),
        _ => todo!(),
    }
}

fn from_url_udp<T: Into<Query> + Send + 'static>(url: Url) -> Box<dyn Session<T>> {
    Box::new(UdpSession { url })
}

fn from_url_http<T: Into<Query> + Send + 'static>(url: Url) -> Box<dyn Session<T>> {
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

#[async_trait]
pub trait Session<T: Into<Query>> {
    async fn send(&self, message: T) -> Result<Bytes>;
}

struct UdpSession {
    url: Url,
}

#[async_trait]
impl<T: Into<Query> + Send + 'static> Session<T> for UdpSession {
    async fn send(&self, _message: T) -> Result<Bytes> {
        let slice = Vec::new();
        let res = Bytes(slice);
        Ok(res)
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
    async fn send_message(&self, message: impl Into<Query>) -> Result<Bytes> {
        let query = Into::<Query>::into(message);
        let mut url = self.url.clone();

        let query_str = Into::<String>::into(query);
        url.set_query(Some(&query_str));

        let uri = url.as_str().parse::<hyper::Uri>()?;
        let req = hyper::Request::get(uri).body(Body::empty())?;

        let response = self.client.request(req).await?;

        self.handle_response(response).await
    }

    async fn handle_response(&self, response: Response<Body>) -> Result<Bytes> {
        let status_code = response.status();

        match status_code {
            StatusCode::OK => Self::process_body(response).await,
            StatusCode::BAD_REQUEST => Err(ConnectionError::Custom("HTTP Bad request".to_string())),
            StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT => self.handle_redirect(response).await,
            _ => Self::process_body(response).await,
        }
    }

    async fn handle_redirect(&self, response: Response<Body>) -> Result<Bytes> {

        Self::process_body(response).await
    }

    async fn process_body(response: Response<Body>) -> Result<Bytes> {
        let body = response.into_body().data().await;
        match body {
            Some(data) => {
                let res = data?.to_vec();

                Ok(Bytes(res))
            }
            None => return Err(ConnectionError::Custom("Received empty Body".to_string())),
        }
    }
}

#[async_trait]
impl<T, K> Session<T> for HttpSession<K>
where
    T: Into<Query> + Send + 'static,
    K: Connect + Clone + Send + Sync + 'static,
{
    async fn send(&self, message: T) -> Result<Bytes> {
        Ok(self.send_message(message).await?)
    }
}

fn build_https_client() -> Client<HttpsConnector<hyper::client::HttpConnector>> {
    let client = Client::builder();
    client.build::<_, Body>(HttpsConnector::new())
}

#[cfg(test)]
mod tests {}
