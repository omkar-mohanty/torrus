use metainfo::Metainfo;
use std::{error::Error, net::SocketAddr};
use torrus::{metainfo, tracker};
use tracker::get_trackers;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub async fn get_peers(info: &Metainfo) -> Result<Vec<SocketAddr>> {
    let trackers = get_trackers(&info)?;

    let mut peers = Vec::new();
    for mut tracker in trackers {
        let mut rsp = tracker.announce().await?;

        peers.append(&mut rsp.peers.addrs);
    }

    Ok(peers)
}
