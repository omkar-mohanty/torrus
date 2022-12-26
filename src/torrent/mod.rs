use crate::metainfo::Metainfo;

use std::sync::Arc;

/// High level manager struct which manages the torrent swarm.
pub struct Torrent {
    metainfo: Arc<Metainfo>,
}
