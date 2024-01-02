use async_trait::async_trait;

use super::Metainfo;

/// This is where the magic happens.
///
/// Drives the state of multiple Torrents.
/// Handles multiple peers
/// Informs the client about events.
#[async_trait]
pub trait Engine: Send + Sync {
    fn add_torrent(&self, metainfo: Metainfo);
    async fn run(&self);
}
