pub mod error;

use async_trait::async_trait;
use byteorder::{ByteOrder, NetworkEndian};
use bytes::{Buf, BytesMut};
use error::ConnectionError;
use hyper::body::HttpBody;
use hyper::{client::connect::Connect, Body, Client};
use hyper::{Response, StatusCode};
use hyper_tls::HttpsConnector;
use rand::Rng;
use std::collections::HashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::ops::Deref;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use tokio_util::codec::{Decoder, Encoder};
use url::Url;

type Result<T> = std::result::Result<T, ConnectionError>;

pub struct Query(HashMap<String, String>);
pub struct Bytes(Vec<u8>);

struct UdpConnectRequest {
    /// 64-bit Protocol ID
    protocol_id: u64,
    /// 32-bit action i.e 'Connect' or 'Announce'
    action: u32,
    /// 64-bit Transaction Id
    transaction_id: u32,
}

impl Into<BytesMut> for UdpConnectRequest {
    fn into(self) -> BytesMut {
        let mut bytes = BytesMut::new();

        let mut buf = [0; 16];
        NetworkEndian::write_u64(&mut buf, self.protocol_id);
        NetworkEndian::write_u32(&mut buf, self.action);
        NetworkEndian::write_u32(&mut buf, self.transaction_id);

        bytes.extend_from_slice(&buf);
        bytes
    }
}

impl UdpConnectRequest {
    fn new() -> Self {
        let transaction_id = rand::thread_rng().gen();

        let protocol_id = 0x4172710198;

        let action = 0;

        Self {
            protocol_id,
            action,
            transaction_id,
        }
    }
}

struct UdpConnectResponse {
    /// 32-bit action i.e 'Connect' or 'Announce'
    action: u32,
    /// 64-bit Transaction Id
    transaction_id: u32,
    /// 64-bit Transaction Id
    connection_id: u64,
}

struct UdpMessageCodec;

impl Encoder<UdpConnectRequest> for UdpMessageCodec {
    type Error = tokio::io::Error;

    fn encode(&mut self, item: UdpConnectRequest, dst: &mut BytesMut) -> tokio::io::Result<()> {
        let mut buf = [0; 16];
        NetworkEndian::write_u64(&mut buf, item.protocol_id);
        NetworkEndian::write_u32(&mut buf, item.action);
        NetworkEndian::write_u32(&mut buf, item.transaction_id);
        dst.extend_from_slice(&buf);
        Ok(())
    }
}

impl Decoder for UdpMessageCodec {
    type Error = tokio::io::Error;

    type Item = UdpConnectResponse;

    fn decode(&mut self, src: &mut BytesMut) -> tokio::io::Result<Option<Self::Item>> {
        if src.len() < 16 {
            return Ok(None);
        }

        let action = src.get_u32();
        let transaction_id = src.get_u32();
        let connection_id = src.get_u64();

        let response = UdpConnectResponse {
            action,
            transaction_id,
            connection_id,
        };

        Ok(Some(response))
    }
}

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

        for (key, value) in map.iter() {
            if res.is_empty() {
                res += key;
                res += "=";
                res += value;
            }

            res += "&";
            res += key;
            res += "=";
            res += value;
        }
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
    Box::new(UdpSession {
        url,
        connection_id: None,
        transaction_id: None,
    })
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
    async fn send(&mut self, message: T) -> Result<Bytes>;
}

struct UdpSession {
    url: Url,
    connection_id: Option<u64>,
    transaction_id: Option<u32>,
}

impl UdpSession {
    async fn connect(&mut self) -> Result<()> {
        let socket_addrs = self.url.socket_addrs(|| None)?;

        for addr in socket_addrs.iter() {
            let response = Self::connect_to_addr(*addr).await?;

            self.transaction_id = Some(response.transaction_id);

            self.connection_id = Some(response.connection_id);
        }

        Ok(())
    }

    async fn connect_to_addr(addr: SocketAddr) -> Result<UdpConnectResponse> {
        let socket = UdpSocket::bind("0.0.0.0:8080").await?;

        for n in 0..=8 {
            let t = 15 * 2_u64.pow(n);

            let timeout_duration = Duration::from_secs(t);

            let request = UdpConnectRequest::new();

            let request_bytes = Into::<BytesMut>::into(request);

            socket.send_to(&request_bytes, addr).await?;

            let mut buf = BytesMut::new();
            match timeout(timeout_duration, socket.recv(&mut buf)).await {
                Ok(res) => {
                    if let Ok(_) = res {
                        let response = Self::parse_response_message(buf);
                        return Ok(response);
                    }

                    continue;
                }

                Err(_) => continue,
            }
        }

        Err(ConnectionError::Custom("Timed out".to_string()))
    }

    fn parse_response_message(buf: BytesMut) -> UdpConnectResponse {
        let mut cursor = Cursor::new(buf);

        let action = cursor.get_u32();
        let transaction_id = cursor.get_u32();
        let connection_id = cursor.get_u64();

        UdpConnectResponse {
            action,
            transaction_id,
            connection_id,
        }
    }
}

#[async_trait]
impl<T> Session<T> for UdpSession
where
    T: Into<Query> + Send + 'static,
{
    async fn send(&mut self, _message: T) -> Result<Bytes> {
        if let (None, None) = (self.transaction_id, self.connection_id) {
            self.connect().await?;
        }

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
            StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT => {
                self.handle_redirect(response).await
            }
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
    async fn send(&mut self, message: T) -> Result<Bytes> {
        Ok(self.send_message(message).await?)
    }
}

fn build_https_client() -> Client<HttpsConnector<hyper::client::HttpConnector>> {
    let client = Client::builder();
    client.build::<_, Body>(HttpsConnector::new())
}

#[cfg(test)]
mod tests {}
