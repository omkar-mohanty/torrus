use futures::{future::join_all, FutureExt, SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::message::Handshake;
use crate::peer::{message_codec::HandShakeCodec, PeerHandler};
use crate::storage::TorrentFile;
use crate::tracker::Tracker;
use crate::{metainfo::Metainfo, piece::PieceHandler};
use crate::{Peer, PeerId, Result};
use std::sync::RwLock;

type RwFiles = Vec<RwLock<TorrentFile>>;

/// Abstracts over the ways of finding peers in a swarm
struct Discovery {
    /// A optional Vec of trackers in case any present
    trackers: Vec<Tracker>,
    /// 20 Byte PeerID of the client
    peer_id: PeerId,
}

impl Discovery {
    pub fn new(trackers: Vec<Tracker>, peer_id: PeerId) -> Self {
        Self { trackers, peer_id }
    }

    pub async fn get_peers(&mut self) -> Result<Vec<Peer>> {
        let mut peers = Vec::new();

        let responses = join_all(
            self.trackers
                .iter_mut()
                .map(|tracker| tracker.announce(self.peer_id).boxed())
                .collect::<Vec<_>>(),
        )
        .await;

        for response in responses {
            let response = match response {
                Ok(res) => {
                    res
                }
                Err(_) => continue,
            };

            let peer_addrs = &response.peers.addrs;

            peers.extend(peer_addrs);
        }

        Ok(peers)
    }
}

/// High level manager struct which manages the torrent swarm.
pub struct Torrent {
    /// Torrent Metadata
    metainfo: Arc<Metainfo>,
    /// Abstraction for getting peers
    discovery: Discovery,
    /// 20 Byte PeerID of the client
    peer_id: PeerId,
    /// Handles Peer in a torrent
    peer_handler: PeerHandler,
}

impl Torrent {
    pub fn from_metainfo(metainfo: Metainfo, peer_id: PeerId) -> Result<Self> {
        let bitfield = crate::Bitfield::with_capacity(metainfo.total_pieces());

        let files: RwFiles = metainfo
            .get_files()
            .iter()
            .map(|file_info| {
                let torrent_file = TorrentFile::new(file_info.to_owned()).unwrap();

                RwLock::new(torrent_file)
            })
            .collect();

        let piece_handler = Arc::new(RwLock::new(PieceHandler::from_metainfo(
            &metainfo, bitfield, files,
        )));

        let metainfo = Arc::new(metainfo);

        let trackers = if let Some(url) = &metainfo.announce {
            let tracker = Tracker::from_url_string(url, Arc::clone(&metainfo))?;

            vec![tracker]
        } else if let Some(al) = &metainfo.announce_list {
            let mut trackers = Vec::new();

            for a in al {
                let tracker = Tracker::from_url_string(&a[0], Arc::clone(&metainfo))?;
                trackers.push(tracker)
            }

            trackers
        } else {
            vec![]
        };

        let discovery = Discovery::new(trackers, peer_id);

        let peer_handler = PeerHandler::new(piece_handler);

        Ok(Self {
            metainfo,
            peer_handler,
            discovery,
            peer_id,
        })
    }

    /// Start Bittorrent `Handshake` protocol with all the peers and then start the wire protocol.
    pub async fn start(&mut self) -> Result<()> {
        let peers = self.discovery.get_peers().await?;

        let results = join_all(
            peers
                .iter()
                .map(|peer| async {
                    let metainfo = Arc::clone(&self.metainfo);

                    let result = connect_to_peer(*peer, metainfo, self.peer_id).await?;

                    Ok::<(Handshake, TcpStream), Box<dyn std::error::Error>>(result)
                })
                .collect::<Vec<_>>(),
        )
        .await;

        for result in results {
            let (id, stream) = if let Ok(result) = result {
                let (handshake, stream) = result;

                let id = handshake.peer_id;

                (id, stream)
            } else {
                continue;
            };

            self.peer_handler.insert_peers(id, stream)
        }

        unimplemented!("Start peer handler");
    }
}
async fn connect_to_peer(
    peer: SocketAddr,
    metainfo: Arc<Metainfo>,
    peer_id: PeerId,
) -> Result<(Handshake, TcpStream)> {
    let stream = TcpStream::connect(peer).await?;

    let mut framed = Framed::new(stream, HandShakeCodec);

    let info_hash = metainfo.hash()?;
    let handshake = Handshake::new(peer_id, info_hash);

    framed.send(handshake).await?;

    loop {
        if let Some(res) = framed.next().await {
            let stream = framed.into_parts().io;
            return Ok((res?, stream));
        } else {
            continue;
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{new_peer_id, Result};

    const FILEPATH: &str = "./resources/ubuntu-22.10-desktop-amd64.iso.torrent";

    #[tokio::test]
    async fn create_from_metainfo() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        let metainfo = Metainfo::from_bytes(&file).unwrap();
        let peer_id = new_peer_id();
        let _ = Torrent::from_metainfo(metainfo, peer_id);

        Ok(())
    }
}
