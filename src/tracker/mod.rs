use std::str::FromStr;

use crate::metainfo::Torrent;
use announce::{http_announce, AnnounceRequestBuilder};
use rand::Rng;
use url::Url;

mod announce;

pub async fn get_peers(torrent: Torrent) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(al) = &torrent.announce_list {
        for a in al {
            let url = Url::parse(a[0].as_str())?;

            match url.scheme() {
                "http" | "https" => {
                    handle_http_tracker(url, &torrent).await?;
                }
                "udp" => {}
                _ => {}
            }
        }
    }

    Ok(())
}

async fn handle_http_tracker(
    url: Url,
    torrent: &Torrent,
) -> Result<(), Box<dyn std::error::Error>> {
    let info_hash = torrent.info.hash()?;
    let peer_id_slice = rand::thread_rng().gen::<[u8; 20]>();

    let mut peer_id = Vec::new();
    peer_id.extend_from_slice(&peer_id_slice);

    let left = torrent.info.length;

    let request = AnnounceRequestBuilder::new()
        .info_hash(info_hash)
        .peer_id(peer_id)
        .with_port(6881)
        .downloaded(0)
        .uploaded(0)
        .left(left)
        .event(String::from_str("started")?)
        .build();

    http_announce::announce(request, url).await?;

    Ok(())
}
