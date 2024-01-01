use crate::TableOfContents;
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use serde_derive::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    result::Result,
    str::FromStr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// Directory for storing .torrent files.
///
/// This is necessary because we cannot trust the OS or user to preserve the .torrent files, so in
/// case the .torrent file is deleted the download can progress.
const APP_DIR: &str = "./torrents";

/// Default directory where all downloaded files will be stored. can be overwritten via [Config]
const DOWNLOAD_DIR: &str = "./downloads";

/// Default implementation of a thread safe client [Client]
pub(crate) static DEFAULT_CLIENT: OnceCell<LockedClient<TorrentClient>> = OnceCell::new();

pub fn init() -> crate::Result<()> {
    DEFAULT_CLIENT.get_or_init(|| LockedClient::new(TorrentClient::new()));
    DEFAULT_CLIENT.get().unwrap().init()?;
    Ok(())
}

/// My interpretation of a client.
///
/// It's not perfect.
pub struct LockedClient<T>(RwLock<T>);

impl<T> LockedClient<T> {
    pub fn new(inner: T) -> Self {
        LockedClient(RwLock::new(inner))
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        self.0.read().unwrap()
    }

    pub fn write(&self) -> RwLockWriteGuard<T> {
        self.0.write().unwrap()
    }
}

pub(crate) struct TorrentClient {
    config: Config,
    toc: TableOfContents,
}

impl TorrentClient {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            toc: TableOfContents::default(),
        }
    }
}

#[async_trait]
impl Client for LockedClient<TorrentClient> {
    type Err = crate::error::TorrErr;

    fn add_torrent(&self, torrent_file: PathBuf) -> Result<(), Self::Err> {
        let data = fs::read(torrent_file)?;
        self.write().toc.add_torrent(&data).unwrap();
        Ok(())
    }

    async fn run(&self) -> Result<(), Self::Err> {
        todo!()
    }

    fn init(&self) -> Result<(), Self::Err> {
        let config = self.get_config();

        if !config.app_dir.exists() {
            std::fs::create_dir(config.app_dir).unwrap();
        }
        if !config.download_dir.exists() {
            std::fs::create_dir(config.download_dir).unwrap();
        }

        Ok(())
    }

    fn set_config(&self, config: Config) {
        self.write().config = config;
    }

    fn get_config(&self) -> Config {
        self.read().config.clone()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    download_dir: PathBuf,
    app_dir: PathBuf,
}

impl Default for Config {
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

    fn init(&self) -> Result<(), Self::Err>;

    /// Run the underlying [Engine] and drive the state machine forward.
    async fn run(&self) -> Result<(), Self::Err>;

    fn get_config(&self) -> Config;
    fn set_config(&self, config: Config);

    /// Read the torrent file to the end and parse it. May or may not check the file validity or
    /// existence.
    fn add_torrent(&self, torrent_file: PathBuf) -> Result<(), Self::Err>;
}

#[cfg(test)]
mod tests {
    const DEFAULT_RESOURCES: &str = "./resources";
    const TEST_DOWNLOAD_DIR: &str = "./tests";
    const TEST_APP_DIR: &str = "./app_dir";

    use super::*;

    fn get_test_config() -> Config {
        Config {
            download_dir: PathBuf::from_str(TEST_DOWNLOAD_DIR).unwrap(),
            app_dir: PathBuf::from_str(TEST_APP_DIR).unwrap(),
        }
    }

    #[test]
    fn test_client() -> crate::Result<()> {
        let client = LockedClient::new(TorrentClient::new());

        for entry in fs::read_dir(DEFAULT_RESOURCES)? {
            let entry = entry?;
            client.add_torrent(entry.path())?;
        }
        Ok(())
    }

    #[test]
    fn test_config() -> crate::Result<()> {
        let config = get_test_config();

        let client = LockedClient::new(TorrentClient::new());
        client.set_config(config);
        client.init()?;

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
