use anyhow;
use std::fs;
use torrus_core::{metainfo::Metainfo, prelude::Sha1Hash};
use torrus_tracker::Tracker;

#[tokio::test]
async fn test_http_tracker() -> anyhow::Result<()> {
    let bytes = fs::read("../resources/ubuntu-22.10-desktop-amd64.iso.torrent")?;
    let metainfo = Metainfo::new(&bytes)?;

    if let Some(announce_url) = metainfo.announce {
        let mut tracker = Tracker::new(&announce_url);
        let id = metainfo.info.as_sha1();
        let response = match tracker.announce(id).await {
            Ok(response) => response,
            Err(msg) => anyhow::bail!("Error {msg}"),
        };
        println!("{:?}", response);
    } else {
        println!("{}", metainfo);
        if let Some(al) = metainfo.announce_list {
            for a in al {
                let mut tracker = Tracker::new(&a[0]);
                let id = metainfo.info.as_sha1();
                let response = match tracker.announce(id).await {
                    Ok(response) => response,
                    Err(msg) => anyhow::bail!("Error {msg}"),
                };
                println!("{:?}", response);
            }
        }
    }

    Ok(())
}
