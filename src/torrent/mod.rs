use std::sync::Arc;

use crate::storage::TorrentFile;
use crate::{metainfo::Metainfo, piece::PieceTracker, Hash};
use tokio::sync::RwLock;

/// High level manager struct which manages the torrent swarm.
pub struct Torrent {
    metainfo: Arc<Metainfo>,
    piece_tracker: PieceTracker,
    files: Vec<RwLock<TorrentFile>>,
    piece_hashes_concat: Hash,
}

impl Torrent {
    pub fn from_metainfo(metainfo: Metainfo) -> Self {
        let metainfo = Arc::new(metainfo);
        
        let bitfield = crate::Bitfield::new();

        let piece_tracker = PieceTracker::new(bitfield);

        let files = metainfo.get_files().iter().map(|file_info| {

            let torrent_file = TorrentFile::new(file_info.to_owned()).unwrap();

            RwLock::new(torrent_file)

        }).collect();

        let piece_hashes_concat = metainfo.info.pieces.to_vec(); 

        Self { metainfo, piece_tracker, files, piece_hashes_concat }

    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;

    const FILEPATH: &str = "./resources/ubuntu-22.10-desktop-amd64.iso.torrent";

    #[tokio::test]
    async fn create_from_metainfo() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        let metainfo = Metainfo::from_bytes(&file).unwrap();

        let _ = Torrent::from_metainfo(metainfo);

        Ok(())
    }
}
