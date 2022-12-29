use std::ops::Deref;

use crate::PieceIndex;

pub(crate) const BLOCK_SIZE: u32 = 0x4000;

pub type BlockData = Vec<u8>;
/// Represents a block of data sent between peers
#[derive(Debug, Clone)]
pub struct Block {
    /// Represents the information about the block
    pub block_info: BlockInfo,
    /// The data within a block
    pub data: BlockData,
}

impl Block {
    pub fn new(block_info: BlockInfo, data: impl Into<BlockData>) -> Self {
        let data = Into::<BlockData>::into(data);
        Self { block_info, data }
    }

    pub fn piece_index(&self) -> PieceIndex {
        self.block_info.piece_index
    }

    pub fn get_offset(&self) -> u32 {
        self.block_info.begin
    }
}

impl Deref for Block {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    /// Index of the piece within the bitfield.
    pub piece_index: PieceIndex,
    /// The offset in bytes within int 'Piece'
    pub begin: u32,
}
