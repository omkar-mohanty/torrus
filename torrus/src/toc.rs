use uuid::Uuid;

use crate::{Metainfo, Result};
use std::collections::HashMap;

/// Global state of the app
pub struct TableOfContents {
    torrents: HashMap<Uuid, Metainfo>,
}

impl TableOfContents {
    ///Parse the [Metainfo] and add it to the Table Of contents.
    pub fn add_torrent(&mut self, id: Uuid, data: &[u8]) -> Result<()> {
        // Parse the torrent
        let torrent = Metainfo::new(data)?;
        self.torrents.insert(id, torrent);
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
