use serde_bencode::to_bytes;
use serde_bytes::ByteBuf;
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Debug, Deserialize)]
pub struct Node(String, i64);

#[derive(Debug, Deserialize, Serialize)]
pub struct File {
    path: Vec<String>,
    length: i64,
    #[serde(default)]
    md5sum: Option<String>,
}

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
    #[serde(flatten)]
    pub files_and_path: Option<FilesAndPath>,
    #[serde(default)]
    pub private: Option<u8>,
    #[serde(default)]
    #[serde(rename = "root hash")]
    pub root_hash: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FilesAndPath {
    files: Vec<File>,
    path: Vec<String>,
}

impl Info {
    pub fn hash(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let bencod_string = to_bytes(self)?;

        let mut hasher = Sha1::new();

        hasher.update(bencod_string);

        let hashed = hasher.finalize().to_owned().to_vec();

        Ok(hashed)
    }
}

#[derive(Debug, Deserialize)]
pub struct Torrent {
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

impl Torrent {
    pub fn from_bytes(v: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_bencode::de::from_bytes::<Torrent>(v)?)
    }
}

pub fn render_torrent(torrent: &Torrent) {
    println!("name:\t\t{}", torrent.info.name);
    println!("announce:\t{:?}", torrent.announce);
    println!("nodes:\t\t{:?}", torrent.nodes);
    if let Some(al) = &torrent.announce_list {
        for a in al {
            println!("announce list:\t{}", a[0]);
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

    use super::*;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    const FILEPATH: &str = "./torrents/ubuntu-22.10-desktop-amd64.iso.torrent";

    #[tokio::test]
    async fn test_from_bytes() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        Torrent::from_bytes(&file)?;
        Ok(())
    }
}
