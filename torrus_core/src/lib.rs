pub mod block;
pub mod id;
pub mod metainfo;
pub mod peer;

pub mod prelude {
    pub use super::block::*;
    pub use super::id::ID;
    pub use super::metainfo::{Info, Metainfo};
    pub use super::peer::*;

    pub trait Sha1Hash {
        fn into_sha1(&self) -> ID;
    }
}
