use error::Result;
use std::collections::HashMap;
use torrent::Metainfo;

pub mod client;
pub mod error;
mod torrent;

pub use client::init;

/// Global state of the app
struct TableOfContents {
    torrents: HashMap<String, Metainfo>,
}

impl TableOfContents {
    ///Parse the [Metainfo] and add it to the Table Of contents.
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
