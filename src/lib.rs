use bitvec::{prelude::Msb0, vec::BitVec};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub mod error;
pub mod metainfo;
pub mod peer;
pub mod storage;
pub mod torrent;
pub mod tracker;
pub mod message;

mod piece;
mod block;
mod dht;

type Hash = Vec<u8>;
type PeerId = [u8; 20];

/// 0 indexed Bitfield which represents the pieces which the client has and does not have
type PieceIndex = usize;
type Bitfield = BitVec<u8, Msb0>;
type Sender = UnboundedSender<message::Message>;
type Receiver = UnboundedReceiver<message::Message>;
type IoResult<T> = tokio::io::Result<T>;

pub type Result<T> = std::result::Result<T, error::TorrusError>;
