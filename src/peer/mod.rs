mod message;
mod session;
mod state;

use message::*;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub(crate) type Sender = UnboundedSender<PeerCodec>;
pub(crate) type Receiver = UnboundedReceiver<PeerCodec>;
pub(crate) type IoResult<T> = tokio::io::Result<T>;

/// The client can initiate a 'Outbound' connection
/// The peer can initiate an 'Inbound' connection
pub enum Direction {
    Inbound,
    Outbound,
}
