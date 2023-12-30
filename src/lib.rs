use error::Result;
use once_cell::sync::OnceCell;
use std::{collections::HashMap, io::{Read, Write}, path::PathBuf, str::FromStr, sync::Mutex, fs};
use torrent::Torrent;

mod error;
mod torrent;

pub static GLOBAL_STATE: OnceCell<Mutex<TableOfContents>> = OnceCell::new();
const DEFAULT_INFOHASH_PATHS: &'static str = "./torrents";

/// Global state of the app
pub struct TableOfContents {
    torrents: HashMap<String, Torrent>,
}

impl TableOfContents {
    pub fn add_torrent(&mut self, info_file_path: PathBuf) -> Result<()> {
        use fs::File;
        let mut file = File::options().read(true).open(&info_file_path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let torrent = Torrent::new(&buf)?;
        self.torrents.insert(String::from_str("")?, torrent);
        let info_hash_store = PathBuf::from_str(DEFAULT_INFOHASH_PATHS).unwrap().join(info_file_path);
        let mut file = File::options().write(true).open(info_hash_store)?;
        file.write_all(&buf)?;
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

pub fn init() -> Result<()> {
    let mut toc = TableOfContents::default();
    let info_hash_path = PathBuf::from_str(DEFAULT_INFOHASH_PATHS).unwrap();

    let metadata = fs::metadata(DEFAULT_INFOHASH_PATHS).unwrap();
    if metadata.is_dir() {
        for entry in fs::read_dir(info_hash_path).unwrap() {
            let entry = entry?;
            let metadata = &entry.metadata()?;
            if metadata.is_file() {
                toc.add_torrent(entry.path())?;
            }
        }
    } else {
        fs::create_dir(DEFAULT_INFOHASH_PATHS).unwrap();
    }

    GLOBAL_STATE.get_or_init(|| Mutex::new(toc));
    Ok(())
}
