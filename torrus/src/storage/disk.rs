use super::{piece::PieceManager, Block, Blockinfo};
use std::collections::HashMap;
use uuid::Uuid;
use async_trait::async_trait;

pub fn default_store() -> impl Store {
    todo!()
}


/// interface for storage
///
/// I am assuming I will not be satisfied with my initial implementation and keeping that in mind
/// it's best not to couple Engine code with Storage code, so interfaces is the way to go.
#[async_trait]
pub trait Store: Send + Sync {
    async fn new_store(&self, id: Uuid); 
    async fn put_block(&self, id: Uuid, block: Block);
    async fn get_block(&self, id: Uuid, block_info: Blockinfo) -> Block;
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
