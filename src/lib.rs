use bitvec::{vec::BitVec, prelude::Msb0};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};

pub mod metainfo;
pub mod peer;
pub mod piece_io;
pub mod torrent;
pub mod tracker;
pub mod error;

mod message;
mod piece;
mod block;

pub type Hash = Vec<u8>;
pub type PeerId = Vec<u8>;

/// 0 indexed Bitfield which represents the pieces which the client has and does not have
pub type PieceIndex = usize;
pub type Bitfield = BitVec<u8, Msb0>;
pub(crate) type Sender = UnboundedSender<message::Message>;
pub(crate) type Receiver = UnboundedReceiver<message::Message>;
pub(crate) type IoResult<T> = tokio::io::Result<T>;


type Result<T> = std::result::Result<T, error::TorrusError>;
