use super::{piece::PieceManager, Block, Blockinfo};
use crate::{torrent::Metainfo, Locked};
use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

pub fn default_store() -> impl Store {
    Locked::new(TorrentStorage::new())
}

/// interface for storage
///
/// I am assuming I will not be satisfied with my initial implementation and keeping that in mind
/// it's best not to couple Engine code with Storage code, so interfaces is the way to go.
#[async_trait]
pub trait Store: Send + Sync {
    async fn new_store(&self, id: Uuid, metainfo: &Metainfo);
    async fn put_block(&self, id: Uuid, block: Block);
    async fn get_block(&self, id: Uuid, block_info: Blockinfo) -> Block;
}

#[async_trait]
impl Store for Locked<TorrentStorage> {
    async fn new_store(&self, id: Uuid, metainfo: &Metainfo) {
        self.write()
            .await
            .managers
            .insert(id, PieceManager::new(metainfo));
    }

    async fn put_block(&self, id: Uuid, block: Block) {
        todo!()
    }

    async fn get_block(&self, id: Uuid, block_info: Blockinfo) -> Block {
        todo!()
    }
}

/// Responsible for writing data to the disk.
///
/// Current implementation holds the Piece in memory until it is completely downloaded, then it
/// flushs the piece to the disk.
///
/// For multi file torrents, [Piece] operations become a bit complicated since [TorrentStorage]
/// needs to figure out if a piece overlaps over multiple files.
pub struct TorrentStorage {
    managers: HashMap<Uuid, PieceManager>,
}

impl TorrentStorage {
    pub fn new() -> Self {
        Self {
            managers: HashMap::new(),
        }
    }
}
