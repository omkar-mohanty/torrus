use crate::Peer;
use std::collections::HashSet;
use tokio::task::JoinHandle;
use torrus_core::{metainfo::Metainfo, prelude::ID};
use torrus_tracker::Tracker;

pub type DefCmd<T, U> = Box<dyn Fn(T) -> U>;

impl<T, Args> Command<Args, T> for DefCmd<Args, T> {}

pub trait Command<Args, T>: Fn(Args) -> T {}

pub struct Engine {
    commands: Vec<Box<dyn Command<(), (), Output = ()>>>,
    torrents: HashSet<ID, TorrentEntry>,
    engine_thread: Option<JoinHandle<()>>,
}

pub struct TorrentEntry {
    metainfo: Metainfo,
    info_hash: ID,
    peers: HashSet<ID, Peer>,
    trackers: Vec<Tracker>,
}
