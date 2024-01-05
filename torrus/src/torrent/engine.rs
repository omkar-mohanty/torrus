use super::Metainfo;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::{
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::JoinHandle,
};
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
pub trait Engine: Sync + Send {
    async fn add_torrent(&self, id: Uuid, metainfo: Metainfo);
    async fn run(&self);
    async fn stop(self);
}

struct LockedEngine<T>(Arc<RwLock<T>>);

impl<T> LockedEngine<T> {
    pub fn new(inner: T) -> Self {
        LockedEngine(Arc::new(RwLock::new(inner)))
    }

    pub async fn read(&self) -> RwLockReadGuard<T> {
        self.0.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<T> {
        self.0.write().await
    }
}

pub enum EngineCommand {
    AddTorrent(Uuid),
}

struct ClientEngine {
    torrents: HashMap<Uuid, Metainfo>,
    join_handle: Option<JoinHandle<()>>,
    sender: Option<UnboundedSender<EngineCommand>>,
}

async fn handle_engine_events(
    engine: Arc<RwLock<ClientEngine>>,
    receiver: UnboundedReceiver<EngineCommand>,
) {
    use EngineCommand::*;
    let mut receiver = receiver;
    while let Some(msg) = receiver.recv().await {
        match msg {
            AddTorrent(id) => {
                let metainfo = engine.read().await.torrents.get(&id).unwrap();
            }
        }
    }
}

impl ClientEngine {
    fn new() -> Self {
        ClientEngine {
            torrents: HashMap::new(),
            join_handle: None,
            sender: None,
        }
    }

    fn add_torrent(&mut self, id: Uuid, metainfo: Metainfo) {
        log::debug!("ID : {id}, Torrent : {metainfo}");
        match self.torrents.insert(id, metainfo) {
            Some(_val) => log::info!("Trying to add an existing torrent!"),
            None => log::info!("Successfully added torrent"),
        };
    }
}

#[async_trait]
impl Engine for LockedEngine<ClientEngine> {
    async fn add_torrent(&self, id: Uuid, metainfo: Metainfo) {
        self.write().await.add_torrent(id, metainfo);
    }

    async fn run(&self) {
        let client_arc = Arc::clone(&self.0);
        let (sender, receiver) = unbounded_channel();
        let join_handle = tokio::task::spawn(async move {
            handle_engine_events(client_arc, receiver).await;
        });
        let mut client_write = self.write().await;
        client_write.sender = Some(sender);
        client_write.join_handle = Some(join_handle);
    }

    async fn stop(self) {
        let client = self.write().await;
        client.join_handle.as_ref().unwrap().abort();
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
        }

        async fn stop(self) {
        
        }
    }
}
