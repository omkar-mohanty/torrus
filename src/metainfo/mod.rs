use serde_bencode::to_bytes;
use serde_bytes::ByteBuf;
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Debug, Deserialize)]
pub struct Node(String, i64);

#[derive(Debug, Deserialize, Serialize)]
pub struct File {
    pub path: Vec<String>,
    pub length: u64,
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
    pub files: Option<Vec<File>>,
    #[serde(default)]
    pub private: Option<u8>,
    #[serde(default)]
    #[serde(rename = "root hash")]
    pub root_hash: Option<String>,
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
    pub fn from_bytes(v: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_bencode::de::from_bytes::<Metainfo>(v)?)
    }

    pub fn total_pieces(&self) -> u64 {
        let total_pieces = if let Some(files) = &self.info.files {
            let mut total_length: u64 = 0;
            for file in files {
                total_length += file.length;
            }

            total_length / self.info.piece_length
        } else {
            self.info.length / self.info.piece_length
        };

        total_pieces
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
                println!("File\t {}", path);
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

    use super::*;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    const FILEPATH: &str = "./resources/ubuntu-22.10-desktop-amd64.iso.torrent";

    #[tokio::test]
    async fn test_from_bytes() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        Metainfo::from_bytes(&file)?;
        Ok(())
    }
}
