pub mod message_codec;
mod session;
mod state;
mod handler;

use message_codec::PeerCodec;
pub use handler::PeerHandler;

use crate::error::TorrusError as PeerError;

/// The client can initiate a 'Outbound' connection
/// The peer can initiate an 'Inbound' connection
pub enum Direction {
    Inbound,
    Outbound,
}

pub use session::PeerSession;
