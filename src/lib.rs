use std::net::SocketAddr;

use bitvec::{prelude::Msb0, vec::BitVec};
use rand::{thread_rng, Rng};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub mod error;
pub mod message;
pub mod metainfo;
pub mod peer;
pub mod storage;
pub mod torrent;
pub mod tracker;

mod block;
mod client;
mod dht;
mod piece;
mod utils;

type Hash = Vec<u8>;
type PeerId = [u8; 20];
type Peer = SocketAddr;

/// 0 indexed Bitfield which represents the pieces which the client has and does not have
type PieceIndex = usize;
/// Byte Offset
type Offset = usize;
type Bitfield = BitVec<u8, Msb0>;
type Sender = UnboundedSender<message::Message>;
type Receiver = UnboundedReceiver<message::Message>;
type IoResult<T> = tokio::io::Result<T>;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn new_peer_id() -> PeerId {
    thread_rng().gen::<PeerId>()
}

pub use client::Client;
