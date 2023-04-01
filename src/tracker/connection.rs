use crate::{Result, TorrusError};

use async_trait::async_trait;
use bytes::{Buf, BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use hyper::body::HttpBody;
use hyper::{client::connect::Connect, Body, Client};
use hyper::{Response, StatusCode};
use hyper_tls::HttpsConnector;
use rand::Rng;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use tokio::net::UdpSocket;
use tokio_util::codec::{Decoder, Encoder};
use tokio_util::udp::UdpFramed;
use url::Url;

use super::{Peers, TrackerRequest, TrackerResponse};

pub struct Query(HashMap<String, String>);
pub struct Bytes(Vec<u8>);

#[derive(Clone)]
struct UdpRequest {
    /// 64-bit Protocol ID
    protocol_id: u64,
    /// 32-bit action i.e 'Connect' or 'Announce'
    action: u32,
    /// 64-bit Transaction Id
    transaction_id: u32,
    /// Tracker Request if there is any
    tracker_request: Option<TrackerRequest>,
    /// Connection ID if already connected to tracker
    connection_id: Option<u64>,
}

impl UdpRequest {
    fn new() -> Self {
        let transaction_id = rand::thread_rng().gen();

        let protocol_id = 0x41727101980;

        let action = 0;

        Self {
            protocol_id,
            action,
            transaction_id,
            tracker_request: None,
            connection_id: None,
        }
    }
}

impl UdpRequest {
    fn with_tracker_request(mut self, tracker_request: TrackerRequest) -> Self {
        self.tracker_request = Some(tracker_request);

        self
    }

    fn with_connection_id(mut self, connection_id: u64) -> Self {
        self.connection_id = Some(connection_id);

        self
    }

    fn with_action(mut self, action: u32) -> Self {
        self.action = action;

        self
    }
}

struct UdpResponse {
    /// 32-bit action i.e 'Connect' or 'Announce'
    #[allow(dead_code)]
    pub action: u32,
    /// 64-bit Transaction Id
    pub transaction_id: u32,
    /// 64-bit Transaction Id
    pub connection_id: Option<u64>,
    /// Tracker response
    pub tracker_response: Option<TrackerResponse>,
}

struct UdpMessageCodec;

fn encode_tracker_request(tracker_request: TrackerRequest, dst: &mut BytesMut) {
    dst.put_slice(&tracker_request.info_hash);
    dst.put_slice(&tracker_request.peer_id);
    dst.put_u64(tracker_request.downloaded);
    dst.put_u64(tracker_request.left);
    dst.put_u64(tracker_request.uploaded);

    let event = tracker_request.event.as_str();
    match event {
        "none" => dst.put_u32(0),
        "completed" => dst.put_u32(1),
        "started" => dst.put_u32(2),
        "stopped" => dst.put_u32(3),
        _ => dst.put_u32(0),
    };

    dst.put_u32(tracker_request.ip_address);
    dst.put_u32(tracker_request.key);
    dst.put_i32(tracker_request.num_want);
    dst.put_u16(tracker_request.port);
}

impl Encoder<UdpRequest> for UdpMessageCodec {
    type Error = tokio::io::Error;

    fn encode(&mut self, item: UdpRequest, dst: &mut BytesMut) -> tokio::io::Result<()> {
        let UdpRequest {
            protocol_id,
            action,
            transaction_id,
            tracker_request,
            connection_id,
        } = item;

        if action == 0 {
            dst.put_u64(protocol_id);
            log::debug!("\tPut Protocol ID:");
        } else if let Some(connection_id) = connection_id {
            dst.put_u64(connection_id);
            log::debug!("\tPut connection ID");
        }
        dst.put_u32(action);
        dst.put_u32(transaction_id);

        if let Some(tracker_request) = tracker_request {
            encode_tracker_request(tracker_request, dst);
            assert!(dst.len() >= 20);
        }
        Ok(())
    }
}

fn decode_ip_address(src: &mut BytesMut) -> Peers {
    let mut addrs = Vec::new();

    while src.remaining() >= 6 {
        let mut ip: [u8; 4] = [0; 4];

        src.copy_to_slice(&mut ip);

        let port = src.get_u16();

        let ip_addr = IpAddr::from(ip);

        let sock = SocketAddr::new(ip_addr, port);

        addrs.push(sock);
    }

    Peers { addrs }
}

fn decode_tracker_response(src: &mut BytesMut) -> TrackerResponse {
    let interval = src.get_u32();
    let leechers = src.get_u32();
    let seeders = src.get_u32();
    let peers = decode_ip_address(src);

    TrackerResponse {
        failure_reason: None,
        warning_message: None,
        complete: seeders,
        interval,
        min_interval: None,
        tracker_id: None,
        incomplete: leechers,
        peers,
    }
}

impl Decoder for UdpMessageCodec {
    type Error = TorrusError;

    type Item = UdpResponse;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        let response = if src.len() < 16 {
            None
        } else {
            let action = src.get_u32();

            let parse_common = |src: &mut BytesMut| {
                let transaction_id = src.get_u32();
                let connection_id = Some(src.get_u64());
                (transaction_id, connection_id)
            };
            match action {
                0 => {
                    let (transaction_id, connection_id) = parse_common(src);

                    Some(UdpResponse {
                        transaction_id,
                        connection_id,
                        action,
                        tracker_response: None,
                    })
                }
                1 => {
                    let transaction_id = src.get_u32();

                    let tracker_response = Some(decode_tracker_response(src));

                    Some(UdpResponse {
                        action,
                        transaction_id,
                        connection_id: None,
                        tracker_response,
                    })
                }
                3 => {
                    let _transaction_id = src.get_u32();
                    let err_message = String::from_utf8_lossy(src).to_string();

                    return Err(TorrusError::new(&err_message));
                }
                _ => None,
            }
        };

        Ok(response)
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

pub fn from_url<'a>(url: &'a Url) -> Box<dyn Session<TrackerRequest> + Send + 'a> {
    match url.scheme() {
        "http" | "https" => from_url_http(url),
        "udp" => from_url_udp(url),
        _ => todo!(),
    }
}

fn from_url_udp<'a>(url: &'a Url) -> Box<dyn Session<TrackerRequest> + Send + 'a> {
    Box::new(UdpSession {
        url,
        connection_id: None,
        transaction_id: None,
    })
}

fn from_url_http<'a>(url: &'a Url) -> Box<dyn Session<TrackerRequest> + Send + 'a> {
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

/// A tracker can be either use http or udp to communicate with clients
/// `Session` abstracts over the Protocol and provides a clean interface to communicate with the
/// tracker
#[async_trait]
pub trait Session<T: Into<Query>> {
    async fn send(&mut self, message: TrackerRequest) -> Result<TrackerResponse>;
}

struct UdpSession<'a> {
    url: &'a Url,
    connection_id: Option<u64>,
    transaction_id: Option<u32>,
}

impl<'a> UdpSession<'a> {
    async fn connect(&mut self) -> Result<()> {
        let socket_addrs = self.url.socket_addrs(|| None)?;

        for addr in socket_addrs.iter() {
            let request = UdpRequest::new();

            let (response, _) = Self::connect_to_addr(*addr, request).await?;

            self.transaction_id = Some(response.transaction_id);

            self.connection_id = response.connection_id;
        }

        Ok(())
    }

    async fn get_socket(addr: &SocketAddr) -> Result<UdpSocket> {
        match addr {
            SocketAddr::V6(addr) => {
                let sock_addr = SocketAddr::new("::1".parse().unwrap(), 0);
                let socket = UdpSocket::bind(sock_addr).await?;
                socket.connect(addr).await?;
                Ok(socket)
            }
            SocketAddr::V4(addr) => {
                let socket = UdpSocket::bind("0.0.0.0:0").await?;
                socket.connect(addr).await?;
                Ok(socket)
            }
        }
    }

    async fn connect_to_addr(
        addr: SocketAddr,
        request: UdpRequest,
    ) -> Result<(UdpResponse, SocketAddr)> {
        let socket = Self::get_socket(&addr).await?;

        let mut socket = UdpFramed::new(socket, UdpMessageCodec);

        socket.send((request.clone(), addr)).await?;

        let mut result = socket.next().await;

        while result.is_none() {
            result = socket.next().await;
        }

        match result.unwrap() {
            Ok(result) => Ok(result),
            Err(err) => Err(TorrusError::new(&err.to_string())),
        }
    }

    async fn send_message(&self, tracker_request: TrackerRequest) -> Result<UdpResponse> {
        let socket_addr = self.url.socket_addrs(|| None)?;

        if let Some(addr) = socket_addr.iter().next() {
            let request = UdpRequest::new()
                .with_action(1)
                .with_connection_id(self.connection_id.unwrap())
                .with_tracker_request(tracker_request.clone());

            let (response, _) = Self::connect_to_addr(*addr, request).await?;

            return Ok(response);
        }

        Err(TorrusError::new("Could not resolve url"))
    }
}

#[async_trait]
impl<'a, T> Session<T> for UdpSession<'a>
where
    T: Into<Query> + Send + 'static,
{
    async fn send(&mut self, message: TrackerRequest) -> Result<TrackerResponse> {
        if let (None, None) = (self.transaction_id, self.connection_id) {
            if let Err(err) = self.connect().await {
                log::error!("\t{}", err);
                return Err(err);
            }
        }

        let response = match self.send_message(message).await {
            Ok(res) => res,
            Err(err) => {
                log::error!("\tsend_message:\t{}", err);
                return Err(err);
            }
        };

        if response.tracker_response.is_none() {
            return Err(TorrusError::new("Could not get tracker response"));
        };

        let tracker_response = response.tracker_response.unwrap();

        Ok(tracker_response)
    }
}

struct HttpSession<'a, T> {
    url: &'a Url,
    client: Client<T>,
}

impl<'a, T> HttpSession<'a, T>
where
    T: Connect + Clone + Send + Sync + 'static,
{
    async fn send_message(&self, message: impl Into<Query>) -> Result<TrackerResponse> {
        let query = Into::<Query>::into(message);
        let mut url = self.url.clone();

        let query_str = Into::<String>::into(query);
        url.set_query(Some(&query_str));

        let uri = url.as_str().parse::<hyper::Uri>()?;
        let req = hyper::Request::get(uri).body(Body::empty())?;

        let response = self.client.request(req).await?;

        let response_bytes = self.handle_response(response).await?;

        let response = serde_bencode::de::from_bytes(&response_bytes)?;

        Ok(response)
    }

    async fn handle_response(&self, response: Response<Body>) -> Result<Bytes> {
        let status_code = response.status();

        match status_code {
            StatusCode::OK => Self::process_body(response).await,
            StatusCode::BAD_REQUEST => Err(TorrusError::new("HTTP Bad request")),
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
            None => Err(TorrusError::new("Received empty Body")),
        }
    }
}

#[async_trait]
impl<'a, T, K> Session<T> for HttpSession<'a, K>
where
    T: Into<Query> + Send + 'static,
    K: Connect + Clone + Send + Sync + 'static,
{
    async fn send(&mut self, message: TrackerRequest) -> Result<TrackerResponse> {
        Ok(self.send_message(message).await?)
    }
}

fn build_https_client() -> Client<HttpsConnector<hyper::client::HttpConnector>> {
    let client = Client::builder();
    client.build::<_, Body>(HttpsConnector::new())
}

#[cfg(test)]
mod tests {}
