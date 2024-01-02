use error::Result;
use torrent::Metainfo;

pub mod client;
pub mod error;

mod storage;
mod toc;
mod torrent;

pub use client::init;
pub use toc::TableOfContents;
