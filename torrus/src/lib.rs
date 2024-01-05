use error::Result;
use torrent::Metainfo;

pub mod client;
pub mod error;

mod storage;
mod toc;
mod torrent;

pub(crate) use toc::TableOfContents;
pub use torrent::default_engine;
