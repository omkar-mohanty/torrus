use std::{collections::HashMap, path::PathBuf};

use tokio::{sync::mpsc::unbounded_channel, task::JoinHandle};

use crate::{
    error::TorrusError,
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
    pub torrent_info: TorrentInfo,
    sender: TorrentCommandSender,
    pub join_handle: JoinHandle<Result<()>>,
}

impl TorrentHandle {
    pub fn send(&self, command: TorrentCommand) -> Result<()> {
        if let Err(err) = self.sender.send(command) {
            return Err(TorrusError::new(&err.to_string()));
        }
        Ok(())
    }
}

/// Torrent Client which manages torrents and is initiated from the entry point
pub struct Client {
    torrents: HashMap<TorrentId, TorrentHandle>,
    client_id: PeerId,
}

impl Client {
    pub fn new() -> Self {
        let client_id = new_peer_id();

        let torrents = HashMap::new();

        Self {
            torrents,
            client_id,
        }
    }

    pub fn add_torrent_from_metainfo(&mut self, metainfo: Metainfo) -> Result<()> {
        let id = metainfo.hash()?;
        let name = metainfo.info.name.clone();

        let created_by = metainfo.created_by.clone();

        let download_dir = metainfo.download_dir.clone();

        let torrent_info = TorrentInfo {
            name,
            created_by,
            download_dir,
        };

        let (sender, receiver) = unbounded_channel();

        let mut torrent = Torrent::from_metainfo(metainfo, self.client_id)?;

        let join_handle = tokio::spawn(async move {
            torrent.start(receiver).await?;

            Ok::<(), TorrusError>(())
        });

        let handle = TorrentHandle {
            torrent_info,
            sender,
            join_handle,
        };

        self.torrents.insert(id, handle);

        Ok(())
    }

    pub async fn run(&self) {
        loop {}
    }

    pub fn list_torrents(&self) {
        for (_, handle) in self.torrents.iter() {
            println!("Name:\t{}", handle.torrent_info.name);
            println!(
                "Download Directory:\t{:?}",
                handle.torrent_info.download_dir
            );
            println!("Created By:\t{:?}", handle.torrent_info.created_by);
        }
    }

    pub fn is_finished(&self, torrent_id: TorrentId) -> bool {
        let torrent = &self.torrents.get(&torrent_id).unwrap();

        torrent.join_handle.is_finished()
    }

    pub fn send_command(&self, torrent_id: TorrentId, command: TorrentCommand) -> Result<()> {
        let torrent = &self.torrents.get(&torrent_id).unwrap();

        torrent.send(command)
    }
}

impl Default for Client {
    fn default() -> Self {
        Client::new() 
    }
}
