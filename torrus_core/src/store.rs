use crate::prelude::{Block, Blockinfo, ID};
use std::error::Error;

pub trait Store {
    type Err: Error;
    fn new_store(&mut self, id: ID) -> Result<(), Self::Err>;
    fn put_block(&mut self, id: ID, block: Block) -> Result<(), Self::Err>;
    fn get_block(&self, id: ID, block_info: Blockinfo) -> Option<Block>;
}
