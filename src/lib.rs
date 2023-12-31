use error::Result;
use std::{collections::HashMap, fs, path::PathBuf, str::FromStr, sync::RwLock};
use torrent::Metainfo;

pub mod client;
pub mod error;
mod torrent;

const DEFAULT_INFOHASH_PATHS: &'static str = "./torrents";

/// Global state of the app
struct TableOfContents {
    torrents: HashMap<String, Metainfo>,
}

impl TableOfContents {
    /// Add torrent to the Table Of contents.
    ///
    /// Also keep another copy of the .torrent file in [DEFAULT_INFOHASH_PATHS] incase the original one gets deleted.
    ///
    /// This method does not check if the file path exists. It's the responsibility of the caller
    /// to do so.
    pub fn add_torrent(&mut self, data: &[u8]) -> Result<()> {
        // Parse the torrent
        let torrent = Metainfo::new(data)?;
        self.torrents.insert(torrent.info.name.clone(), torrent);
        Ok(())
    }
}

impl Default for TableOfContents {
    fn default() -> Self {
        Self {
            torrents: HashMap::default(),
        }
    }
}

/// Initialize the [TableOfContents]. Check the [DEFAULT_INFOHASH_PATHS] if it has .torrent files
/// if it does add it to the TOC.
///
/// If the [DEFAULT_INFOHASH_PATHS] does not exist create the directory
pub fn init() -> Result<()> {
    let mut toc = TableOfContents::default();
    let info_hash_path = PathBuf::from_str(DEFAULT_INFOHASH_PATHS).unwrap();

    let metadata = fs::metadata(DEFAULT_INFOHASH_PATHS).unwrap();
    if metadata.is_dir() {
        for entry in fs::read_dir(info_hash_path).unwrap() {
            let entry = entry?;
            let metadata = &entry.metadata()?;
            if metadata.is_file() {
                let data = fs::read(entry.path())?;
                toc.add_torrent(&data)?;
            }
        }
    } else {
        fs::create_dir(DEFAULT_INFOHASH_PATHS).unwrap();
    }
    Ok(())
}
