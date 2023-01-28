use std::fmt::Display;

use crate::{block::Block, error::TorrusError, Bitfield, Hash, PeerId, PieceIndex};
/// Struct representing the 'Handshake' of the Bittorrent protocol
pub struct Handshake {
    pub peer_id: PeerId,
    pub info_hash: Hash,
    pub reserved: [u8; 8],
}

impl Handshake {
    pub fn new(peer_id: PeerId, info_hash: Hash) -> Self {
        Self {
            peer_id,
            info_hash,
            reserved: [0; 8],
        }
    }
    pub fn len() -> usize {
        19 + 49
    }
}

/// All messages described in Bittorrent wire Protocol
#[derive(Debug, Clone)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(PieceIndex),
    Bitfield(Bitfield),
    Request {
        index: PieceIndex,
        begin: u32,
        length: u32,
    },
    Piece(Block),
    Cancel {
        index: PieceIndex,
        begin: u32,
        length: u32,
    },
    Port(u16),
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Message::*;
        match self {
            KeepAlive => f.write_str("KeepAlive"),
            Choke => f.write_str("Choke"),
            Unchoke => f.write_str("Unchoke"),
            Interested => f.write_str("Interested"),
            NotInterested => f.write_str("Not Interested"),
            Have(index) => f.write_fmt(format_args!("Have Index : {}", index)),
            Bitfield(bitfield) => f.write_fmt(format_args!("Bitfield : {}", bitfield.len())),
            Request {
                index,
                begin,
                length,
            } => f.write_fmt(format_args!(
                "Request Index : {}, Begin : {}, Length : {}",
                index, begin, length
            )),
            Piece(block) => f.write_fmt(format_args!(
                "Piece index : {}, begin : {}",
                block.block_info.piece_index, block.block_info.begin
            )),
            Cancel {
                index,
                begin,
                length,
            } => f.write_fmt(format_args!(
                "Request Index : {}, Begin : {}, Length : {}",
                index, begin, length
            )),
            Port(port) => f.write_fmt(format_args!("Port : {}", port)),
        }
    }
}

#[repr(u8)]
pub enum MessageID {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Port = 9,
}

impl TryFrom<u8> for MessageID {
    type Error = TorrusError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use MessageID::*;
        match value {
            0 => Ok(Choke),
            1 => Ok(Unchoke),
            2 => Ok(Interested),
            3 => Ok(NotInterested),
            4 => Ok(Have),
            5 => Ok(Bitfield),
            6 => Ok(Request),
            7 => Ok(Piece),
            8 => Ok(Cancel),
            9 => Ok(Port),
            _ => Err(TorrusError::new("Unkown message ID")),
        }
    }
}
