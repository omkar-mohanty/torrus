use std::{
    path::PathBuf,
    result::Result,
    str::FromStr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard}, fs,
};

use crate::TableOfContents;

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

struct TorrentClient {
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

impl Client for LockedClient<TorrentClient> {
    type Err = crate::error::TorrErr;
    fn add_torrent(&self,torrent_file: PathBuf) -> Result<(), Self::Err> {
        let data = fs::read(torrent_file).unwrap();
        self.write().toc.add_torrent(&data).unwrap();
        Ok(())
    }
    fn run(&self) -> Result<(), Self::Err> {
        todo!()
    }
    fn set_config(&self, config: Config) {
        self.write().config = config;
    }
    fn get_config(&self) -> Config {
        self.read().config.clone()
    }
}

#[derive(Clone)]
pub struct Config {
    download_dir: PathBuf,
    app_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from_str("./downloads").unwrap(),
            app_dir: PathBuf::from_str("./torrents").unwrap(),
        }
    }
}

pub trait Client {
    type Err: std::error::Error;
    fn run(&self) -> Result<(), Self::Err>;
    fn get_config(&self) -> Config;
    fn set_config(&self, config: Config);
    fn add_torrent(&self,torrent_file: PathBuf) -> Result<(), Self::Err>;
}
