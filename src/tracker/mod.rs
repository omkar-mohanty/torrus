use crate::metainfo::Torrent;
use byteorder::ByteOrder;
use rand::Rng;
use serde::Deserializer;
use serde_bytes::ByteBuf;
use serde_derive::Deserialize;
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    str::FromStr,
};
use url::Url;

use connection::Connection;

mod announce;
mod connection;

pub enum TrackerState {
    Alive,
    Dead,
}

pub struct Tracker {
    id: Option<ByteBuf>,
    alive: TrackerState,
    connection: Connection
}

pub async fn get_peers(torrent: Torrent) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(al) = &torrent.announce_list {
        for a in al {
            let url = Url::parse(a[0].as_str())?;

            match url.scheme() {
                "http" | "https" => {
                    handle_http_tracker(url, &torrent).await?;
                }
                "udp" => {}
                _ => {}
            }
        }
    }

    Ok(())
}

async fn handle_http_tracker(
    url: Url,
    torrent: &Torrent,
) -> Result<(), Box<dyn std::error::Error>> {
    let info_hash = torrent.info.hash()?;
    let peer_id_slice = rand::thread_rng().gen::<[u8; 20]>();

    let mut peer_id = Vec::new();
    peer_id.extend_from_slice(&peer_id_slice);

    let left = torrent.info.length;

    let request = TrackerRequestBuilder::new()
        .info_hash(info_hash)
        .peer_id(peer_id)
        .with_port(6881)
        .downloaded(0)
        .uploaded(0)
        .left(left)
        .event(String::from_str("started")?)
        .build();

    //announce::http_announce::announce(request, url).await?;

    Ok(())
}

#[derive(Clone, Debug)]
pub struct TrackerRequest {
    // Fields for the announce request
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
    downloaded: u64,
    left: u64,
    uploaded: u64,
    event: String,
    ip_address: u32,
    key: u32,
    num_want: i32,
    port: u16,
}

#[derive(Deserialize, Debug)]
pub struct DictPeer {
    #[serde(deserialize_with = "deserialize_ip_string")]
    ip: IpAddr,
    #[allow(dead_code)]
    peer_id: Option<ByteBuf>,
    port: u16,
}

impl DictPeer {
    fn to_socketaddr(&self) -> SocketAddr {
        SocketAddr::new(self.ip, self.port)
    }
}

fn deserialize_ip_string<'de, D>(de: D) -> Result<IpAddr, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = IpAddr;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expecting a Ipv4 or Ipv6 address")
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            IpAddr::from_str(v).map_err(|e| E::custom(format!("Could not parse ip: {}", e)))
        }
        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let ip_str = String::from_utf8_lossy(v);
            match IpAddr::from_str(&ip_str) {
                Ok(ip) => Ok(ip),
                Err(_) => Err(E::custom("Could not parse ip")),
            }
        }
    }
    de.deserialize_str(Visitor {})
}

fn parse_compact_peers(b: &[u8]) -> Vec<SocketAddrV4> {
    let mut ips = Vec::new();

    for chunk in b.chunks_exact(6) {
        let ip = &chunk[..4];
        let port_chunk = &chunk[4..6];
        let ipaddr = Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
        let port = byteorder::BigEndian::read_u16(port_chunk);
        ips.push(SocketAddrV4::new(ipaddr, port))
    }

    ips
}

#[derive(Debug, Deserialize)]
pub struct TrackerResponse {
    // Human readable message for why the request failed
    #[serde(rename = "failure reason")]
    pub failure_reason: Option<ByteBuf>,
    #[serde(rename = "warning message")]
    pub warning_message: Option<ByteBuf>,
    pub complete: u64,
    pub interval: u64,
    #[serde(rename = "min interval")]
    pub min_interval: Option<u64>,
    pub tracker_id: Option<ByteBuf>,
    pub incomplete: u64,
    pub peers: Peers,
}

#[derive(Debug)]
pub struct Peers {
    pub addrs: Vec<SocketAddr>,
}

impl<'de> serde::Deserialize<'de> for Peers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<'de> {
            phantom: std::marker::PhantomData<&'de ()>,
        }

        impl<'de> serde::de::Visitor<'de> for Visitor<'de> {
            type Value = Peers;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a list of peers in dict or binary format")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut peers = Vec::new();
                while let Some(peer) = seq.next_element::<DictPeer>()? {
                    peers.push(peer.to_socketaddr())
                }
                Ok(Peers { addrs: peers })
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Peers {
                    addrs: parse_compact_peers(v)
                        .into_iter()
                        .map(|v| v.into())
                        .collect(),
                })
            }
        }

        deserializer.deserialize_any(Visitor {
            phantom: PhantomData,
        })
    }
}

impl std::fmt::Display for TrackerResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(failure_reason) = &self.failure_reason {
            let reason = String::from_utf8_lossy(failure_reason.as_slice());

            return f.write_str(&reason);
        }
        if let Some(warning_message) = &self.warning_message {
            let message = String::from_utf8_lossy(warning_message.as_slice());
            let message_str = format!("Warning:\t{}", &message);
            f.write_str(&message_str)?;
        }

        let response_str = format!(
            "Completed:\t{}\nInterval:\t{}\nIncomplete:\t{}\n",
            &self.complete, &self.interval, &self.incomplete
        );

        f.write_str(&response_str)
    }
}

pub struct TrackerRequestBuilder {
    // Fields for the announce request
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
    downloaded: u64,
    left: u64,
    uploaded: u64,
    event: String,
    ip_address: u32,
    key: u32,
    num_want: i32,
    port: u16,
}

impl TrackerRequestBuilder {
    // Constructor for the TrackerRequestBuilder struct
    pub fn new() -> TrackerRequestBuilder {
        TrackerRequestBuilder {
            info_hash: vec![],
            peer_id: vec![],
            downloaded: 0,
            left: 0,
            uploaded: 0,
            event: String::new(),
            ip_address: 0,
            key: 0,
            num_want: 0,
            port: 0,
        }
    }

    // Method to set the info_hash field of the announce request
    pub fn info_hash(mut self, info_hash: Vec<u8>) -> TrackerRequestBuilder {
        self.info_hash = info_hash;
        self
    }

    // Method to set the peer_id field of the announce request
    pub fn peer_id(mut self, peer_id: Vec<u8>) -> TrackerRequestBuilder {
        self.peer_id = peer_id;
        self
    }

    // Method to set the downloaded field of the announce request
    pub fn downloaded(mut self, downloaded: u64) -> TrackerRequestBuilder {
        self.downloaded = downloaded;
        self
    }

    // Method to set the left field of the announce request
    pub fn left(mut self, left: u64) -> TrackerRequestBuilder {
        self.left = left;
        self
    }

    // Method to set the uploaded field of the announce request
    pub fn uploaded(mut self, uploaded: u64) -> TrackerRequestBuilder {
        self.uploaded = uploaded;
        self
    }

    // Method to set the event field of the announce request
    pub fn event(mut self, event: String) -> TrackerRequestBuilder {
        self.event = event;
        self
    }

    // Method to set the ip_address field of the announce request
    pub fn ip_address(mut self, ip_address: u32) -> TrackerRequestBuilder {
        self.ip_address = ip_address;
        self
    }

    // Method to set the key field of the announce request
    pub fn key(mut self, key: u32) -> TrackerRequestBuilder {
        self.key = key;
        self
    }

    // Method to set the num_want field of the announce request
    pub fn num_want(mut self, num_want: i32) -> TrackerRequestBuilder {
        self.num_want = num_want;
        self
    }

    pub fn with_port(mut self, port: u16) -> TrackerRequestBuilder {
        self.port = port;
        self
    }

    pub fn build(self) -> TrackerRequest {
        TrackerRequest {
            info_hash: self.info_hash,
            peer_id: self.peer_id,
            downloaded: self.downloaded,
            left: self.left,
            uploaded: self.uploaded,
            event: self.event,
            ip_address: self.ip_address,
            key: self.key,
            num_want: self.num_want,
            port: self.port,
        }
    }
}
