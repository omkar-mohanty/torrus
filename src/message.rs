use crate::{PeerId, Hash, Bitfield, PieceIndex, block::Block, error::TorrusError};

/// Struct representing the 'Handshake' of the Bittorrent protocol
pub struct Handshake {
    pub peer_id: PeerId,
    pub info_hash: Hash,
}

impl Handshake {
    pub fn len() -> usize {
        19 + 48
    }
}

/// All messages described in Bittorrent wire Protocol
#[derive(Debug)]
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
    Piece {
        index: PieceIndex,
        begin: u32,
        block: Block,
    },
    Cancel {
        index: PieceIndex,
        begin: u32,
        length: u32,
    },
    Port(u16),
}

pub struct PeerCodec;

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


