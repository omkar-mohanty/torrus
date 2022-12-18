mod metainfo;
mod tracker;

use metainfo::{render_torrent, Torrent};
use serde_bencode::de;
use std::{env, error::Error, fs};
use tracker::get_peers;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[tokio::main]
async fn main() -> Result<()> {
    if let Some(path) = env::args().nth(1) {
        let buffer = fs::read(path)?;
        let torrent = de::from_bytes::<Torrent>(&buffer)?;
        if let Some(_) = env::args().nth(2) {
            render_torrent(&torrent);
        }
        get_peers(torrent).await?;
    } else {
        println!("path to file must be there");
    }

    Ok(())
}
