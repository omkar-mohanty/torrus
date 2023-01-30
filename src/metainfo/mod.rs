use std::path::PathBuf;

use serde_bencode::to_bytes;
use serde_bytes::ByteBuf;
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::Result;
use crate::{storage::FileInfo, Hash};

#[derive(Debug, Deserialize)]
pub struct Node(String, i64);

/// Bittorrent spec describes Path as being a list.
/// The Path can be a directory or else it can be a file in which case it is the last element in
/// the list.
#[derive(Debug, Deserialize, Serialize)]
pub struct File {
    pub path: Vec<String>,
    pub length: u64,
    #[serde(default)]
    md5sum: Option<String>,
}

/// Represents Bittorrent Info dictionary
/// If the torrent is single file the `files` field is empty in which case the `name` becomes the
/// path of the torrent
#[derive(Debug, Deserialize, Serialize)]
pub struct Info {
    pub name: String,
    pub pieces: ByteBuf,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde(default)]
    pub md5sum: Option<String>,
    #[serde(default)]
    pub length: u64,
    pub files: Option<Vec<File>>,
    #[serde(default)]
    pub private: Option<u8>,
    #[serde(default)]
    #[serde(rename = "root hash")]
    pub root_hash: Option<String>,
}

impl Info {
    /// 20 Byte SHA - 1 Info hash
    pub fn hash(&self) -> Result<Vec<u8>> {
        let bencod_string = to_bytes(self)?;

        let mut hasher = Sha1::new();

        hasher.update(bencod_string);

        let hashed = hasher.finalize().to_owned().to_vec();

        Ok(hashed)
    }
}

/// Contains all the necessary information to connect to trackers aang getting the infohash
#[derive(Debug, Deserialize)]
pub struct Metainfo {
    /// Info disctionary
    pub info: Info,
    /// Tracker's announce URL
    #[serde(default)]
    pub announce: Option<String>,
    /// If DHT supported the well known nodes
    #[serde(default)]
    pub nodes: Option<Vec<Node>>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub httpseeds: Option<Vec<String>>,
    #[serde(default)]
    #[serde(rename = "announce-list")]
    /// If multi tracker is present the url of all the said trackers
    pub announce_list: Option<Vec<Vec<String>>>,
    #[serde(default)]
    #[serde(rename = "creation date")]
    pub creation_date: Option<i64>,
    #[serde(rename = "comment")]
    pub comment: Option<String>,
    #[serde(default)]
    #[serde(rename = "created by")]
    pub created_by: Option<String>,
    pub download_dir: Option<PathBuf>,
}

impl Metainfo {
    pub fn from_bytes(v: &[u8]) -> Result<Self> {
        Ok(serde_bencode::de::from_bytes::<Metainfo>(v)?)
    }

    pub fn total_pieces(&self) -> usize {
        let total_pieces = if let Some(files) = &self.info.files {
            let mut total_length: u64 = 0;
            for file in files {
                total_length += file.length;
            }

            total_length / self.info.piece_length
        } else {
            self.info.length / self.info.piece_length
        };

        total_pieces as usize
    }

    /// Get TorrentFile metainfo
    pub fn get_files(&self) -> Vec<FileInfo> {
        if let Some(files) = &self.info.files {
            let mut offset = 0;

            files
                .iter()
                .map(|file| {
                    let mut path: PathBuf = PathBuf::new();

                    file.path.iter().for_each(|path_str| {
                        path.push(path_str);
                    });

                    offset += file.length;

                    if let Some(download_dir) = self.download_dir.clone() {
                        path = download_dir.join(path);
                    }

                    FileInfo {
                        path,
                        offset,
                        length: file.length,
                    }
                })
                .collect()
        } else {
            let file_string = &self.info.name;
            let mut path = PathBuf::from(file_string);

            if let Some(download_dir) = self.download_dir.clone() {
                path = download_dir.join(path);
            }

            vec![FileInfo {
                path,
                offset: 0,
                length: self.info.length,
            }]
        }
    }

    pub fn hash(&self) -> Result<Hash> {
        Ok(self.info.hash()?.to_vec())
    }
}

pub fn render_torrent(torrent: &Metainfo) {
    println!("name:\t\t{}", torrent.info.name);
    println!("announce:\t{:?}", torrent.announce);
    println!("nodes:\t\t{:?}", torrent.nodes);
    if let Some(al) = &torrent.announce_list {
        for a in al {
            println!("announce list:\t{}", a[0]);
        }
    }
    for files in torrent.info.files.iter() {
        for file in files.iter() {
            for path in file.path.iter() {
                println!("File\t {path}");
            }
        }
    }
    println!("httpseeds:\t{:?}", torrent.httpseeds);
    println!("creation date:\t{:?}", torrent.creation_date);
    println!("comment:\t{:?}", torrent.comment);
    println!("created by:\t{:?}", torrent.created_by);
    println!("encoding:\t{:?}", torrent.encoding);
    println!("piece length:\t{:?}", torrent.info.piece_length);
    println!("private:\t{:?}", torrent.info.private);
    println!("root hash:\t{:?}", torrent.info.root_hash);
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use crate::storage::TorrentFile;

    use super::*;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    const FILEPATH: &str = "./resources/ubuntu-22.10-desktop-amd64.iso.torrent";
    const FILEPATH_MULTI: &str = "./resources/multi.torrent";

    #[tokio::test]
    async fn test_from_bytes() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        Metainfo::from_bytes(&file)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_file_create_from_metainfo() -> Result<()> {
        let buffer = std::fs::read(FILEPATH)?;

        let metainfo = Metainfo::from_bytes(&buffer)?;

        let files_dwn = metainfo.get_files();

        let files: Vec<Result<()>> = files_dwn
            .iter()
            .map(|file_info| -> Result<()> {
                let path = PathBuf::from("/tmp").join(&file_info.path);

                let file_info = FileInfo {
                    path: path.clone(),
                    offset: file_info.offset,
                    length: file_info.length,
                };

                TorrentFile::new(file_info)?;

                match Path::exists(&path) {
                    true => Ok(()),

                    false => {
                        panic!("File not created")
                    }
                }
            })
            .collect();

        for file in files {
            file?
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_file_create_multi() -> Result<()> {
        let buffer = std::fs::read(FILEPATH_MULTI)?;

        let metainfo = Metainfo::from_bytes(&buffer)?;

        let files_dwn = metainfo.get_files();

        let files: Vec<Result<()>> = files_dwn
            .iter()
            .map(|file_info| -> Result<()> {
                let path = PathBuf::from("/tmp").join(&file_info.path);

                let file_info = FileInfo {
                    path: path.clone(),
                    offset: file_info.offset,
                    length: file_info.length,
                };

                TorrentFile::new(file_info)?;

                match Path::exists(&path) {
                    true => Ok(()),

                    false => {
                        panic!("File not created")
                    }
                }
            })
            .collect();

        for file in files {
            file?
        }
        Ok(())
    }
}
