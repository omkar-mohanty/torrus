use crate::{
    default_engine,
    engine::EngineCommand,
    torrent::{Engine, Metainfo},
    Locked, TableOfContents,
};
use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use std::{fs, path::PathBuf, result::Result, str::FromStr, sync::Arc};
use uuid::Uuid;

/// Directory for storing .torrent files.
///
/// This is necessary because we cannot trust the OS or user to preserve the .torrent files, so in
/// case the .torrent file is deleted the download can progress.
const APP_DIR: &str = "./torrents";

/// Default directory where all downloaded files will be stored. can be overwritten via [Config]
const DOWNLOAD_DIR: &str = "./downloads";

pub fn default_client() -> impl Client {
    Locked::new(TorrentClient::new(default_engine()))
}

pub(crate) struct TorrentClient {
    config: ClientConfig,
    toc: TableOfContents,
    engine: Arc<dyn Engine>,
}

impl TorrentClient {
    pub fn new(engine: impl Engine + 'static) -> Self {
        Self {
            config: ClientConfig::default(),
            toc: TableOfContents::default(),
            engine: Arc::new(engine),
        }
    }
}

#[async_trait]
impl Client for Locked<TorrentClient> {
    type Err = crate::error::TorrErr;

    async fn add_torrent(&self, torrent_file: PathBuf) -> Result<(), Self::Err> {
        let data = fs::read(torrent_file)?;
        let id = Uuid::new_v4();
        let mut client_write = self.write().await;
        client_write.toc.add_torrent(id, &data).unwrap();
        let metinfo = Metainfo::new(&data).unwrap();
        log::info!("Adding torrent");
        println!("Here ");
        client_write
            .engine
            .send_command(EngineCommand::AddTorrent(id, metinfo))
            .await;
        Ok(())
    }

    async fn run(&self) -> Result<(), Self::Err> {
        self.write().await.engine.run().await;
        Ok(())
    }

    async fn init(&self) -> Result<(), Self::Err> {
        let config = self.get_config().await;

        if !config.app_dir.exists() {
            std::fs::create_dir(config.app_dir).unwrap();
        }
        if !config.download_dir.exists() {
            std::fs::create_dir(config.download_dir).unwrap();
        }

        Ok(())
    }

    async fn set_config(&self, config: ClientConfig) -> Result<(), Self::Err> {
        self.write().await.config = config;
        self.init().await?;
        Ok(())
    }

    async fn get_config(&self) -> ClientConfig {
        self.read().await.config.clone()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    download_dir: PathBuf,
    app_dir: PathBuf,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from_str(DOWNLOAD_DIR).unwrap(),
            app_dir: PathBuf::from_str(APP_DIR).unwrap(),
        }
    }
}

#[async_trait]
pub trait Client: Send + Sync {
    type Err: std::error::Error;

    async fn init(&self) -> Result<(), Self::Err>;

    /// Run the underlying [Engine] and drive the state machine forward.
    async fn run(&self) -> Result<(), Self::Err>;

    async fn get_config(&self) -> ClientConfig;
    async fn set_config(&self, config: ClientConfig) -> Result<(), Self::Err>;

    /// Read the torrent file to the end and parse it. May or may not check the file validity or
    /// existence.
    async fn add_torrent(&self, torrent_file: PathBuf) -> Result<(), Self::Err>;
}

#[cfg(test)]
mod tests {
    const DEFAULT_RESOURCES: &str = "./resources";
    const TEST_DOWNLOAD_DIR: &str = "./tests";
    const TEST_APP_DIR: &str = "./app_dir";
    use crate::torrent::engine::tests::TestEngine;

    use super::*;

    fn get_test_config() -> ClientConfig {
        ClientConfig {
            download_dir: PathBuf::from_str(TEST_DOWNLOAD_DIR).unwrap(),
            app_dir: PathBuf::from_str(TEST_APP_DIR).unwrap(),
        }
    }

    #[tokio::test]
    async fn test_client() -> crate::Result<()> {
        let test_engine = TestEngine::new();
        let client = Locked::new(TorrentClient::new(test_engine));

        for entry in fs::read_dir(DEFAULT_RESOURCES)? {
            let entry = entry?;
            client.add_torrent(entry.path()).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_config() -> crate::Result<()> {
        let config = get_test_config();
        let test_engine = TestEngine::new();
        let client = Locked::new(TorrentClient::new(test_engine));
        client.set_config(config).await?;
        client.init().await?;

        if !fs::metadata(TEST_APP_DIR).unwrap().is_dir() {
            panic!("APP directory is not a directory")
        }

        if !fs::metadata(TEST_DOWNLOAD_DIR).unwrap().is_dir() {
            panic!("Download directory is not a directory")
        }

        fs::remove_dir(TEST_APP_DIR).unwrap();
        fs::remove_dir(TEST_DOWNLOAD_DIR).unwrap();

        Ok(())
    }
}
