use error::Result;
use torrent::Metainfo;

pub mod client;
pub mod error;

mod locked;
mod storage;
mod toc;
mod torrent;

pub(crate) use locked::Locked;
pub(crate) use toc::TableOfContents;

pub use torrent::default_engine;
pub use torrent::engine;
