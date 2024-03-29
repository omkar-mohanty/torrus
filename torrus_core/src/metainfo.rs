use anyhow::Result;
use serde_bytes::ByteBuf;
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fmt::Display;

use crate::{id::ID, prelude::Sha1Hash};

#[derive(Debug, Deserialize)]
pub struct Node(String, i64);

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

impl Sha1Hash for Info {
    fn as_sha1(&self) -> crate::id::ID {
        let bytes = serde_bencode::to_bytes(&self).unwrap();

        let mut hasher = Sha1::new();

        hasher.update(bytes);

        let hashed = hasher.finalize().to_owned().to_vec();
        ID::from(hashed)
    }
}

/// V1 Bittorrent metainfo
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Metainfo {
    pub info: Info,
    #[serde(default)]
    pub announce: Option<String>,
    #[serde(default)]
    pub nodes: Option<Vec<Node>>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub httpseeds: Option<Vec<String>>,
    #[serde(default)]
    #[serde(rename = "announce-list")]
    pub announce_list: Option<Vec<Vec<String>>>,
    #[serde(default)]
    #[serde(rename = "creation date")]
    pub creation_date: Option<i64>,
    #[serde(rename = "comment")]
    pub comment: Option<String>,
    #[serde(default)]
    #[serde(rename = "created by")]
    pub created_by: Option<String>,
}

impl Metainfo {
    pub fn new(data: &[u8]) -> Result<Self> {
        Ok(serde_bencode::de::from_bytes::<Metainfo>(data)?)
    }
}

impl Display for Metainfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("name:\t\t{}\n", self.info.name))?;
        f.write_fmt(format_args!("announce:\t\t{:?}\n", self.announce))?;
        f.write_fmt(format_args!("nodes:\t\t{:?}\n", self.nodes))?;
        if let Some(al) = &self.announce_list {
            for a in al {
                f.write_fmt(format_args!("announce list:\t{}\n", a[0]))?;
            }
        }
        f.write_fmt(format_args!("httpsseeds:\t{:?}\n", self.httpseeds))?;
        f.write_fmt(format_args!("creation date:\t{:?}\n", self.creation_date))?;
        f.write_fmt(format_args!("comment:\t{:?}\n", self.comment))?;
        f.write_fmt(format_args!("created by:\t{:?}\n", self.created_by))?;
        f.write_fmt(format_args!("encoding:\t{:?}\n", self.encoding))?;
        f.write_fmt(format_args!(
            "piece length:\t{:?}\n",
            self.info.piece_length
        ))?;
        f.write_fmt(format_args!("private:\t{:?}\n", self.info.private))?;
        f.write_fmt(format_args!("root hash:\t{:?}\n", self.info.root_hash))?;
        f.write_fmt(format_args!("md5sum:\t\t{:?}\n", self.info.md5sum))?;
        if let Some(files) = &self.info.files {
            for file in files {
                f.write_fmt(format_args!("file path:\t{:?}\n", file.path))?;
                f.write_fmt(format_args!("file path:\t{:?}\n", file.length))?;
                f.write_fmt(format_args!("file path:\t{:?}\n", file.md5sum))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::fs;

    #[test]
    fn test_decode() -> Result<()> {
        for entry in fs::read_dir("../resources")? {
            let data = fs::read(entry?.path())?;
            Metainfo::new(&data)?;
        }
        Ok(())
    }

    #[test]
    fn test_encode() -> Result<()> {
        for entry in fs::read_dir("../resources")? {
            let data = fs::read(entry?.path())?;
            let metadata = Metainfo::new(&data)?;
            metadata.info.as_sha1();
        }
        Ok(())
    }
}
