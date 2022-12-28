use super::{PeerError, Receiver, Sender};
use crate::message::{Message, PeerCodec};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_util::codec::Framed;

type Result<T> = std::result::Result<T, PeerError>;

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
    pub fn new(msg_send: Sender) -> (Sender, Self) {
        let (sender, msg_rcv) = mpsc::unbounded_channel::<Message>();

        let peer_session = PeerSession { msg_send, msg_rcv };

        (sender, peer_session)
    }
}

impl PeerSession {
    /// The 'PeerSession' first checks if the connection is 'Outbound' or 'Inbound'.
    ///
    /// If the connection is 'Outbound' then the 'PeerSession' uses 'start_outbound' handler else
    /// the 'start_inbound' handler is used
    pub async fn start(&mut self, stream: TcpStream) -> Result<()> {
        let stream = Framed::new(stream, PeerCodec);

        let (mut sink, mut stream) = stream.split();

        let msg_rcv = &mut self.msg_rcv;
        let msg_send = &mut self.msg_send;

        loop {
            tokio::select! {
                cmd = msg_rcv.recv() => {
                    if let Some(msg) = cmd {
                        sink.send(msg).await?;
                    }
                }

                msg = stream.next() => {
                    if let Some(msg) = msg {
                        let peer_msg = msg?;

                        if let Err(_) = msg_send.send(peer_msg) {
                            println!("Could not send message to client");
                            todo!()
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{net::SocketAddr, str::FromStr};

    use tokio::net::TcpListener;
    use tokio_util::codec::{Decoder, Encoder};

    use super::*;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    impl Encoder<String> for PeerCodec {
        type Error = tokio::io::Error;

        fn encode(&mut self, item: String, dst: &mut bytes::BytesMut) -> tokio::io::Result<()> {
            dst.extend_from_slice(item.as_bytes());
            Ok(())
        }
    }

    async fn start_tcp(port: u16) -> SocketAddr {
        let addr = format!("127.0.0.1:{}", port);

        tokio::spawn(async move {
            let listner = TcpListener::bind("127.0.0.1:8080").await.unwrap();

            let (stream, _) = listner.accept().await.unwrap();

            let stream = Framed::new(stream, PeerCodec);

            let (mut sink, mut stream) = stream.split::<Message>();

            loop {
                tokio::select! {
                    _ = sink.send(Message::Choke) => {

                    }

                    msg = stream.next() => {
                         if let Some(msg)=  msg {

                         let msg = msg.unwrap();
                        println!("Got message");
                        sink.send(msg).await.unwrap();
                    }

                }
                }
            }
        });

        SocketAddr::from_str(&addr).unwrap()
    }

    #[tokio::test]
    async fn test_handshake() -> Result<()> {
        let addr = start_tcp(8080).await;

        let mut stream = TcpStream::connect(addr).await;

        while let Err(_) = stream {
            stream = TcpStream::connect(addr).await;
        }

        let stream = stream.unwrap();

        let (msg_send, mut msg_rcv) = mpsc::unbounded_channel::<Message>();

        let (_, mut session) = PeerSession::new(msg_send);

        tokio::spawn(async move {
            session.start(stream).await.unwrap();
        });

        let mut msg = msg_rcv.recv().await;

        while let None = msg {
            msg = msg_rcv.recv().await;
        }

        let msg = msg.unwrap();
        match msg {
            Message::Choke => return Ok(()),
            _ => panic!("Not corrent message"),
        }
    }
}
