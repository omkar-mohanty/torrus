mod error;
mod message_codec;
mod session;
mod state;

use error::*;
use crate::message::Message;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub(crate) type Sender = UnboundedSender<Message>;
pub(crate) type Receiver = UnboundedReceiver<Message>;
pub(crate) type IoResult<T> = tokio::io::Result<T>;

/// The client can initiate a 'Outbound' connection
/// The peer can initiate an 'Inbound' connection
pub enum Direction {
    Inbound,
    Outbound,
}
