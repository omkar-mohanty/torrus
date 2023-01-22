mod handler;
pub mod message_codec;
mod session;
mod state;

pub use handler::PeerHandler;
use message_codec::PeerCodec;

use crate::error::TorrusError as PeerError;

/// The client can initiate a 'Outbound' connection
/// The peer can initiate an 'Inbound' connection
pub enum Direction {
    Inbound,
    Outbound,
}

pub use session::PeerSession;
