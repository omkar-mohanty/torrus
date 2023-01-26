use std::{collections::HashMap, path::PathBuf};

use tokio::sync::mpsc::unbounded_channel;

use crate::{
    metainfo::Metainfo,
    new_peer_id,
    torrent::{Torrent, TorrentCommandSender},
    PeerId, Result,
};

pub enum TorrentCommand {
    Progress,
}

type TorrentId = crate::Hash;

struct TorrentInfo {
    name: String,
    created_by: Option<String>,
    download_dir: Option<PathBuf>,
}

struct TorrentHandle {
    torrent_info: TorrentInfo,
    sender: TorrentCommandSender,
}

/// Torrent Client which manages torrents and is initiated from the entry point
pub struct Client {
    torrents: HashMap<TorrentId, TorrentHandle>,
    peer_id: PeerId,
}

impl Client {
    pub fn new() -> Self {
        let peer_id = new_peer_id();

        let torrents = HashMap::new();

        Self { torrents, peer_id }
    }

    pub fn add_torrent_from_metainfo(&mut self, metainfo: Metainfo) -> Result<()> {
        let id = metainfo.hash()?;
        let name = metainfo.info.name.clone();

        let created_by = metainfo.created_by.clone();

        let download_dir = metainfo.download_dir;

        let torrent_info = TorrentInfo {
            name,
            created_by,
            download_dir,
        };

        let (sender, receiver) = unbounded_channel();

        let torrent = Torrent::from_metainfo(metainfo, self.peer_id, receiver)?;

        tokio::spawn(async move {
            if let Err(_) = torrent.start().await {
                return;
            }
        });

        let handle = TorrentHandle {
            torrent_info,
            sender,
        };
        self.torrents.insert(id, handle);
        Ok(())
    }

    pub async fn run(self) {
        for mut torrent in self.torrents {}
    }
}
