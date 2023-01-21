use std::collections::HashMap;
use std::sync::Arc;

use futures::{StreamExt, SinkExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::codec::Framed;

use crate::message::Handshake;
use crate::peer::PeerSession;
use crate::peer::message_codec::HandShakeCodec;
use crate::storage::TorrentFile;
use crate::tracker::Tracker;
use crate::{metainfo::Metainfo, piece::PieceHandler};
use crate::{new_peer_id, Peer, PeerId, Receiver, Result, Sender};
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

        for tracker in self.trackers.iter_mut() {
            let response = tracker.announce(self.peer_id.to_vec()).await?;

            peers.extend(response.peers.addrs);
        }

        Ok(peers)
    }
}

/// High level manager struct which manages the torrent swarm.
pub struct Torrent {
    /// Torrent Metadata
    metainfo: Arc<Metainfo>,
    /// Handles reading,writing and keeping track of pieces in a torrent.
    /// May be accessed from multiple threads
    piece_handler: Arc<PieceHandler>,
    /// Abstraction for getting peers 
    discovery: Discovery,
    /// All connected peers to the client
    peer_sessions: HashMap<PeerId, Sender>,
    /// 20 Byte PeerID of the client
    peer_id: Arc<PeerId>,
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

        let piece_handler = Arc::new(PieceHandler::from_metainfo(&metainfo, bitfield, files));

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

        let peer_sessions = HashMap::new();

        let peer_id = Arc::new(peer_id);

        Ok(Self {
            metainfo,
            piece_handler,
            discovery,
            peer_sessions,
            peer_id,
        })
    }

    /// Start Bittorrent `Handshake` protocol with all the peers and then start the wire protocol.
    pub async fn start(&mut self) -> Result<()> {
        let mut peers = self.discovery.get_peers().await?;

        let (sender, receiver) = unbounded_channel();

        let peers = peers.iter().map(|peer| {
            let fut = TcpStream::connect(peer.clone());

            let (sender, mut peer_session) = PeerSession::new(sender.clone());

            let metainfo = Arc::clone(&self.metainfo);

            let peer_id = Arc::clone(&self.peer_id);

            tokio::spawn(async move {
                let stream = match fut.await {
                    Ok(stream) => stream,
                    Err(_) => return,
                };

                let  mut stream = Framed::new(stream, HandShakeCodec);

                let info_hash = metainfo.hash().expect("Could not calculate hash");

                let handshake = Handshake::new(*peer_id, info_hash);

                if let Err(_) = stream.send(handshake).await {
                    return;
                }

                match stream.next().await {
                    Some(res) => {
                        match res {
                            Ok(handshake) => {
                                let id = handshake.peer_id;
                            
                            },
                            Err(_) => {
                                return ;
                            }
                        }

                    }
                    None => {

                    }
                };

            });

            let id = new_peer_id();
            (id, sender)
        }).collect::<HashMap<PeerId, Sender>>();

        self.peer_sessions = peers;

        Ok(())
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
