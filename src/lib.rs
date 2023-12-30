use std::{collections::HashSet, path::PathBuf};
use once_cell::sync::OnceCell;
use torrent::Torrent;
use error::Result;

mod torrent;
mod error;

static GLOBAL_STATE: OnceCell<TableOfContents> = OnceCell::new();
const DEFAULT_INFOHASH_PATHS:&'static str = "./torrents"; 

struct TableOfContents {
    torrents: HashSet<Torrent>
}

impl TableOfContents {
    pub fn add_torrent(&mut self, info_file_path:PathBuf) -> Result<()> {
        Ok(())
    }
}

impl Default for TableOfContents {
    fn default() -> Self {
        Self { torrents: HashSet::default() }
    }
}

pub fn init() {
    let mut toc = TableOfContents::default();
}
