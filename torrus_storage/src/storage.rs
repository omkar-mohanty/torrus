use torrus_core::{block::Blockinfo, id::ID, prelude::Block};
use anyhow::Result;

pub trait Store {
    fn new_store(&mut self, id: ID) -> Result<()>;
    fn put_block(&mut self, id: ID, block: Block) -> Result<()>;
    fn get_block(&self, id: ID, block_info: Blockinfo) -> Option<Block>;
}
