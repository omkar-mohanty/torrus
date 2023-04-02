use super::PeerCodec;
use crate::block::Block;
use crate::message::Message;
use crate::Result;
use crate::TorrusError;
use crate::{Receiver, Sender};
use futures::{SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_util::codec::Framed;

/// Responsible for passing `Message` between client and peer.
pub struct PeerSession {
    /// Request queue to improve performance
    req_queue: Vec<Block>,
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

        let req_queue = Vec::new();

        let peer_session = PeerSession {
            msg_send,
            msg_rcv,
            req_queue,
        };

        (sender, peer_session)
    }
}

impl PeerSession {
    /// Starts listening for messages from client and from the connected peer.
    pub async fn start(&mut self, stream: TcpStream) -> Result<()> {
        let stream = Framed::new(stream, PeerCodec);

        let (mut sink, mut stream) = stream.split();

        let msg_rcv = &mut self.msg_rcv;
        let msg_send = &mut self.msg_send;

        loop {
            tokio::select! {
            cmd = msg_rcv.recv() => {
                if let Some(msg) = cmd {
                    match msg {
                        Message::Piece(block) => {
                            if self.req_queue.len() >=10 {
                                let req_block = self.req_queue.pop().unwrap();

                                sink.send(Message::Piece(req_block)).await?;

                                self.req_queue.push(block);
                            }
                        }
                        _ => {
                            sink.send(msg).await?;
                        }
                    }
                }
            }

            msg = stream.next() => {
                if let Some(msg) = msg {
                    let peer_msg = msg?;

                    if let Err(err) = msg_send.send(peer_msg) {
                        log::error!("Error:\t{err}");
                        return Err(TorrusError::new(&err.to_string()));
                    }
                }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        block::{Block, BlockInfo, BLOCK_SIZE},
        Bitfield,
    };
    use std::{net::SocketAddr, str::FromStr};
    use tokio::net::TcpListener;

    fn get_message(id: u8) -> Message {
        let block_info = BlockInfo {
            piece_index: 12,
            begin: 12,
            length: BLOCK_SIZE,
        };
        match id {
            0 => Message::KeepAlive,
            1 => Message::Choke,
            2 => Message::Unchoke,
            3 => Message::Interested,
            4 => Message::NotInterested,
            5 => Message::Have(12),
            6 => Message::Request(block_info),
            7 => Message::Cancel {
                index: 12,
                begin: 12,
                length: 12,
            },
            8 => Message::Port(8080),
            9 => {
                let block_info = BlockInfo {
                    piece_index: 12,
                    begin: 12,
                    length: BLOCK_SIZE,
                };

                let data = vec![];

                let block = Block::new(block_info, data);

                Message::Piece(block)
            }
            10 => Message::Request(block_info),
            11 => Message::Have(12),
            12 => {
                let bitfield = Bitfield::new();
                Message::Bitfield(bitfield)
            }
            _ => Message::KeepAlive,
        }
    }

    async fn start_tcp(port: u16) -> SocketAddr {
        let addr = format!("127.0.0.1:{}", port);

        tokio::spawn(async move {
            let listner = TcpListener::bind(format!("127.0.0.1:{}", port))
                .await
                .unwrap();

            let (stream, _) = listner.accept().await.unwrap();

            let stream = Framed::new(stream, PeerCodec);

            let (mut sink, mut stream) = stream.split::<Message>();

            loop {
                let mut msg = stream.next().await;
                while let None = msg {
                    msg = stream.next().await;
                }

                let message = msg.unwrap().unwrap();

                sink.send(message).await.unwrap();
            }
        });

        SocketAddr::from_str(&addr).unwrap()
    }

    #[tokio::test]
    async fn test_msg_send() -> Result<()> {
        let addr = start_tcp(8080).await;

        let mut stream = TcpStream::connect(addr).await;

        while let Err(_) = stream {
            stream = TcpStream::connect(addr).await;
        }

        let stream = stream.unwrap();

        let (msg_send, mut msg_rcv) = mpsc::unbounded_channel::<Message>();

        let (cmd_sender, mut session) = PeerSession::new(msg_send);

        tokio::spawn(async move {
            session.start(stream).await.unwrap();
        });

        for id in 0..=12 {
            let message = get_message(id);

            cmd_sender.send(message.clone()).unwrap();

            let mut rsp = msg_rcv.recv().await;

            while let None = rsp {
                rsp = msg_rcv.recv().await;
            }

            let _msg = rsp.unwrap();

            matches!(message, _msg);
        }

        Ok(())
    }
}
