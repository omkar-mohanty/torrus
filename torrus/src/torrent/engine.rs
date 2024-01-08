use super::Metainfo;
use crate::{
    locked::Locked,
    storage::{default_store, Store},
};
use async_trait::async_trait;
use std::sync::Arc;
use std::{collections::HashMap, fmt::Debug};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::{sync::RwLock, task::JoinHandle};
use uuid::Uuid;

pub fn default_engine() -> impl Engine {
    Locked::new(ClientEngine::new())
}

/// This is where the magic happens.
///
/// Drives the state of multiple Torrents.
/// Handles multiple peers
/// Informs the client about events.
#[async_trait]
pub trait Engine: Sync + Send {
    async fn send_command(&self, cmd: EngineCommand);
    async fn run(&self);
    async fn stop(self);
}

pub enum EngineCommand {
    AddTorrent(Uuid, Metainfo),
}

impl Debug for EngineCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use EngineCommand::*;
        match self {
            AddTorrent(id, metainfo) => {
                f.write_fmt(format_args!("ID\t:\t{id}\nMetainfo\t:\t{metainfo}"))
            }
        }
    }
}

struct ClientEngine {
    torrents: HashMap<Uuid, Metainfo>,
    join_handle: Option<JoinHandle<()>>,
    sender: Option<UnboundedSender<EngineCommand>>,
    store: Arc<dyn Store>,
}

async fn handle_engine_events(
    engine: Arc<RwLock<ClientEngine>>,
    receiver: UnboundedReceiver<EngineCommand>,
) {
    use EngineCommand::*;
    let mut receiver = receiver;
    while let Some(msg) = receiver.recv().await {
        match msg {
            AddTorrent(id, metainfo) => {
                let mut engine_write = engine.write().await;
                log::info!("Adding torrent to the engine");
                // TODO engine should create a new store
                engine_write.store.new_store(id, &metainfo).await;
                engine_write.add_torrent(id, metainfo);
            }
        }
    }
}

impl ClientEngine {
    fn new() -> Self {
        let store = default_store();
        ClientEngine {
            torrents: HashMap::new(),
            join_handle: None,
            sender: None,
            store: Arc::new(store),
        }
    }

    fn add_torrent(&mut self, id: Uuid, metainfo: Metainfo) {
        match self.torrents.insert(id, metainfo) {
            Some(_val) => {}
            None => {}
        };
    }
}

#[async_trait]
impl Engine for Locked<ClientEngine> {
    async fn send_command(&self, command: EngineCommand) {
        log::debug!("Sending Command {:?}", command);
        self.write()
            .await
            .sender
            .as_ref()
            .unwrap()
            .send(command)
            .unwrap();
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
        let join_handle = client.join_handle.as_ref().unwrap();

        if !join_handle.is_finished() {
            join_handle.abort();
        }
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
        async fn send_command(&self, _: EngineCommand) {}

        async fn run(&self) {}

        async fn stop(self) {}
    }
}
