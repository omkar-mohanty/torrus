use bitvec::{prelude::Msb0, vec::BitVec};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub mod error;
pub mod metainfo;
pub mod peer;
pub mod storage;
pub mod torrent;
pub mod tracker;

mod block;
mod message;
mod piece;

pub type Hash = Vec<u8>;
pub type PeerId = Vec<u8>;

/// 0 indexed Bitfield which represents the pieces which the client has and does not have
pub type PieceIndex = usize;
pub type Bitfield = BitVec<u8, Msb0>;
pub type Sender = UnboundedSender<message::Message>;
pub type Receiver = UnboundedReceiver<message::Message>;
pub type IoResult<T> = tokio::io::Result<T>;

pub type Result<T> = std::result::Result<T, error::TorrusError>;
