use bitvec::{vec::BitVec, prelude::Msb0};

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
