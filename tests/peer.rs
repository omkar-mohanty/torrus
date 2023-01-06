mod setup;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use rand::{thread_rng, Rng};
use setup::Result;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use torrus::{message::Handshake, peer::message_codec::HandShakeCodec};
const PATH: &str = "./resources/ubuntu-22.10-desktop-amd64.iso.torrent";
#[tokio::test]
async fn test_peer() -> Result<()> {
    let buffer = std::fs::read(PATH)?;
    let torrent = torrus::metainfo::Metainfo::from_bytes(&buffer)?;

    let torrent = Arc::new(torrent);
    let peers = setup::get_peers(&torrent).await?;

    let mut handles = vec![];
    for addr in peers {
        let torrent = Arc::clone(&torrent);
        let info_hash = torrent.info.hash().unwrap();
        let handle = tokio::spawn(async move {
            let stream = match TcpStream::connect(addr).await {
                Ok(stream) => stream,
                Err(err) => return Err(err),
            };

            let mut stream = Framed::new(stream, HandShakeCodec);
            let peer_id = thread_rng().gen::<[u8; 20]>();
            let reserved = [0; 8];
            let handshake = Handshake::new(peer_id, info_hash, reserved);
            if let Err(err) = stream.send(handshake).await {
                println!("Error sending handshake");
                println!("err {}", err);
                return Err(err);
            };

            let mut res = stream.next().await;

            while let None = res {
                res = stream.next().await;
            }

            let msg = res.unwrap().unwrap();
            println!("{:?}", msg.peer_id);
            Ok(())
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await?;
    }
    Ok(())
}
