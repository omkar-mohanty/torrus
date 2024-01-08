use crate::torrent::Metainfo;

use super::Piece;
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

pub struct PieceManager {
    byte_range: Range<u64>,
    /// Cannot keep all the pieces in memory because of large torrent size hence resort to keeping
    /// a few in a cache and when a miss occurs get piece from disk.
    cache: HashMap<usize, Piece>,
    download_directory: PathBuf,
}

impl PieceManager {
    pub fn new(metainfo: &Metainfo) -> Self {
        todo!()
    }
}
