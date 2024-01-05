use std::sync::Arc;

use async_trait::async_trait;

use super::Metainfo;

pub static DEFAULT_ENGINE: ClientEngine = ClientEngine::new();

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

pub struct ClientEngine;

impl ClientEngine {
    const fn new() -> Self {
        ClientEngine
    }
}

#[async_trait]
impl Engine for ClientEngine {
    fn add_torrent(&self, metainfo: Metainfo) {
        todo!()
    }

    async fn run(&self) {
        todo!()
    }
}
