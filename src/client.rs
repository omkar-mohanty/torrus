use crate::{metainfo::Metainfo, new_peer_id, torrent::Torrent, PeerId, Result};

/// Torrent Client which manages torrents and is initiated from the entry point
pub struct Client {
    torrents: Vec<Torrent>,
    peer_id: PeerId,
}

impl Client {
    pub fn new() -> Self {
        let peer_id = new_peer_id();

        let torrents = vec![];
        Self { torrents, peer_id }
    }

    pub fn add_torrent(&mut self, torrent: Torrent) {
        self.torrents.push(torrent)
    }

    pub fn add_torrent_from_metainfo(&mut self, metainfo: Metainfo) -> Result<()> {
        let torrent = Torrent::from_metainfo(metainfo, self.peer_id)?;

        Ok(self.torrents.push(torrent))
    }

    pub async fn run(self) {
        for mut torrent in self.torrents {
           let _ = torrent.start().await;
        }
    }
}
