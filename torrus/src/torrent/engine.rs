use super::Metainfo;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

pub fn default_engine() -> impl Engine {
    LockedEngine::new(ClientEngine::new())
}

/// This is where the magic happens.
///
/// Drives the state of multiple Torrents.
/// Handles multiple peers
/// Informs the client about events.
#[async_trait]
pub trait Engine: Send + Sync {
    async fn add_torrent(&self, id: Uuid, metainfo: Metainfo);
    async fn run(&self);
}

struct LockedEngine<T>(RwLock<T>);

impl<T> LockedEngine<T> {
    pub const fn new(inner: T) -> Self {
        LockedEngine(RwLock::new(inner))
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        self.0.read().unwrap()
    }

    pub fn write(&self) -> RwLockWriteGuard<T> {
        self.0.write().unwrap()
    }
}

struct ClientEngine {
    torrents: HashMap<Uuid, Metainfo>,
}

impl ClientEngine {
    fn new() -> Self {
        ClientEngine {
            torrents: HashMap::new(),
        }
    }

    fn add_torrent(&mut self, id: Uuid, metainfo: Metainfo) {
        log::debug!("ID : {id}, Torrent : {metainfo}");
        match self.torrents.insert(id, metainfo) {
            Some(_val) => log::info!("Trying to add an existing torrent!"),
            None => log::info!("Successfully added torrent")
        };
    }
}

#[async_trait]
impl Engine for LockedEngine<ClientEngine> {
    async fn add_torrent(&self, id: Uuid, metainfo: Metainfo) {
        self.write().add_torrent(id, metainfo);
    }

    async fn run(&self) {
        todo!()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) struct TestEngine;

    impl TestEngine {
        pub fn new() -> TestEngine {
            TestEngine
        }
    }

    #[async_trait::async_trait]
    impl Engine for TestEngine {
        async fn add_torrent(&self, _: Uuid, _: Metainfo) {}

        async fn run(&self) {
            todo!()
        }
    }
}
