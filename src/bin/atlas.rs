use ferrotorr::*;
use metainfo::{render_torrent, Metainfo};
use std::{env, error::Error, fs};
use tracker::get_trackers;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[tokio::main]
async fn main() -> Result<()> {
    if let Some(path) = env::args().nth(1) {
        let buffer = fs::read(path)?;
        let torrent = Metainfo::from_bytes(&buffer)?;
        println!(
            "Total pieces = {} ",
            torrent.info.length / torrent.info.piece_length
        );
        if env::args().nth(2).is_some() {
            render_torrent(&torrent);
        }
        let trackers = get_trackers(&torrent)?;

        for mut tracker in trackers {
            tracker.announce().await?;
        }
    } else {
        println!("path to file must be there");
    }

    Ok(())
}
