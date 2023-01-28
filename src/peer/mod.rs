mod handler;
pub mod message_codec;
mod peer_context;
pub mod session;
mod state;

use message_codec::PeerCodec;

use crate::error::TorrusError as PeerError;

/// The client can initiate a 'Outbound' connection
/// The peer can initiate an 'Inbound' connection
pub enum Direction {
    Inbound,
    Outbound,
}

pub use handler::new_peer;

pub type Peer = Box<dyn self::handler::Peer>;
