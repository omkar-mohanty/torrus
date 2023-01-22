use metainfo::{render_torrent, Metainfo};
use std::{env, error::Error, fs, sync::Arc};
use torrus::tracker::Tracker;
use torrus::*;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[tokio::main]
async fn main() -> Result<()> {
    if let Some(path) = env::args().nth(1) {
        let buffer = fs::read(path)?;
        let metainfo = Metainfo::from_bytes(&buffer)?;
        println!(
            "Total pieces = {} ",
            metainfo.info.length / metainfo.info.piece_length
        );
        if env::args().nth(2).is_some() {
            render_torrent(&metainfo);
        }

        let metainfo = Arc::new(metainfo);

        let trackers = if let Some(url) = &metainfo.announce {
            let tracker = Tracker::from_url_string(url, Arc::clone(&metainfo))?;

            vec![tracker]
        } else if let Some(al) = &metainfo.announce_list {
            let mut trackers = Vec::new();

            for a in al {
                let tracker = Tracker::from_url_string(&a[0], Arc::clone(&metainfo))?;
                trackers.push(tracker)
            }

            trackers
        } else {
            vec![]
        };

        let peer_id = new_peer_id();
        for mut tracker in trackers {
            let resp = tracker.announce(peer_id).await?;
            println!("{:?}", resp.peers.addrs);
        }
    } else {
        println!("path to file must be there");
    }

    Ok(())
}
