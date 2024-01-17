use byteorder::ByteOrder;
use serde::Deserializer;
use serde_bytes::ByteBuf;
use serde_derive::Deserialize;
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    str::FromStr,
};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TrackerResponse {
    Error {
        #[serde(rename = "failure reason")]
        failure_reason: String,
    },
    Response {
        #[serde(rename = "warning message")]
        warning_message: Option<String>,
        complete: u32,
        interval: u32,
        #[serde(rename = "min interval")]
        min_interval: Option<u64>,
        tracker_id: Option<String>,
        incomplete: u32,
        peers: Peers,
    },
}

#[derive(Debug)]
pub struct Peers {
    pub addrs: Vec<DictPeer>,
}

#[derive(Deserialize, Debug)]
pub struct DictPeer {
    #[serde(deserialize_with = "deserialize_ip_string")]
    pub ip: IpAddr,
    #[allow(dead_code)]
    pub peer_id: Option<ByteBuf>,
    pub port: u16,
}

impl From<SocketAddrV4> for DictPeer {
    fn from(value: SocketAddrV4) -> Self {
        let port = value.port();
        let ip = value.ip();

        DictPeer {
            ip: IpAddr::V4(*ip),
            peer_id: None,
            port,
        }
    }
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

fn deserialize_ip_string<'de, D>(de: D) -> std::result::Result<IpAddr, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = IpAddr;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expecting a Ipv4 or Ipv6 address")
        }
        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            IpAddr::from_str(v).map_err(|e| E::custom(format!("Could not parse ip: {e}")))
        }
        fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
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

impl<'de> serde::Deserialize<'de> for Peers {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
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

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut peers = Vec::new();
                while let Some(peer) = seq.next_element::<DictPeer>()? {
                    peers.push(peer)
                }
                Ok(Peers { addrs: peers })
            }

            fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
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
