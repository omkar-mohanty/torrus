use std::sync::Arc;

use crate::storage::TorrentFile;
use crate::tracker::{Tracker, TrackerRequestBuilder, TrackerResponse};
use crate::{metainfo::Metainfo, piece::PieceHandler, Hash};
use crate::{PeerId, Result, Peer};
use std::sync::{Mutex, RwLock};

type RwFiles = Vec<RwLock<TorrentFile>>;

struct Discovery {
    trackers: Vec<Tracker>,
    peer_id: PeerId,
}

impl Discovery {
    pub fn new(trackers: Vec<Tracker>, peer_id: PeerId)-> Self {
        
        Self { trackers, peer_id }
    }

    pub async fn get_peers(&mut self)->Result<Vec<Peer>> {
       let mut peers= Vec::new();

        for tracker in self.trackers.iter_mut() {

           let response = tracker.announce(self.peer_id.to_vec()).await?; 

            peers.extend(response.peers.addrs);
        }

        Ok(peers)
    }
}

/// High level manager struct which manages the torrent swarm.
pub struct Torrent {
    metainfo: Arc<Metainfo>,
    piece_handler: PieceHandler,
    piece_hashes_concat: Hash,
    discovery: Discovery,
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

        let piece_handler = PieceHandler::from_metainfo(&metainfo, bitfield, files);
        let piece_hashes_concat = metainfo.info.pieces.to_vec();

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

        Ok(Self {
            metainfo,
            piece_handler,
            piece_hashes_concat,
            discovery,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        
        let peers = self.discovery.get_peers().await?;
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
