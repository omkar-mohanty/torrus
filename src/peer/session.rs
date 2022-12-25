use super::{IoResult, PeerCodec, Receiver, Sender};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_util::codec::Framed;

/// Responsible for passing `Message` between client and peer.
pub struct PeerSession {
    /// Receiver for commands from the client
    msg_rcv: Receiver,
    /// Sender for Messages
    msg_send: Sender,
}

impl PeerSession {
    /// The client sends a 'Sender' channel to send events back to the client.
    ///
    /// The 'PeerSession' constructor returns a 'Sender' channel so that the client can send
    /// commands to the session.
    ///
    /// This constructor is exclusively used for Outbound connections.
    pub fn new(stream: TcpStream, msg_send: Sender) -> (Sender, Self) {
        let (sender, msg_rcv) = mpsc::unbounded_channel::<PeerCodec>();

        let peer_session = PeerSession { msg_send, msg_rcv };

        (sender, peer_session)
    }
}

impl PeerSession {
    /// The 'PeerSession' first checks if the connection is 'Outbound' or 'Inbound'.
    ///
    /// If the connection is 'Outbound' then the 'PeerSession' uses 'start_outbound' handler else
    /// the 'start_inbound' handler is used
    pub async fn start(&self, stream: TcpStream) -> IoResult<()> {
        let stream = Framed::new(stream, PeerCodec);

        let (mut sink, stream) = stream.split();

        Ok(())
    }
}
