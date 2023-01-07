use crate::storage::TorrentFile;
use crate::{metainfo::Metainfo, piece::PieceTracker, Hash};
use tokio::sync::RwLock;

/// High level manager struct which manages the torrent swarm.
pub struct Torrent {
    metainfo: Metainfo,
    piece_tracker: PieceTracker,
    files: Vec<RwLock<TorrentFile>>,
    piece_hashes_concat: Hash,
}
