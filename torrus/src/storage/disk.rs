use std::collections::HashMap;
use uuid::Uuid;
use super::piece::PieceManager;


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

