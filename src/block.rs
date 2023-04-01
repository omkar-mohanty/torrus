use std::ops::{Deref, Range};

use crate::{storage::IoVec, Offset, PieceIndex};

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

    pub fn get_offset(&self) -> Offset {
        self.block_info.begin as Offset
    }

    /// Range of bytes which the block holds data for
    pub fn byte_range(&self) -> Range<Offset> {
        let start = self.get_offset();

        let end = start + self.data.len();

        Range { start, end }
    }

    pub fn get_slice(&self, range: Range<usize>) -> IoVec {
        let begin = range.start;
        let data = self.data[range].to_vec();
        IoVec::new(begin, data)
    }
}

impl Deref for Block {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.as_ref()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlockInfo {
    /// Index of the piece within the bitfield.
    pub piece_index: PieceIndex,
    /// The offset in bytes within int 'Piece'
    pub begin: u32,
    /// Length of the block
    pub length: u32
}

impl BlockInfo {
    pub fn new(piece_index: PieceIndex, begin: u32) -> Self {
        BlockInfo { piece_index, begin, length: BLOCK_SIZE }
    }
}

pub fn block_count(piece_length: u64) -> usize {
    (piece_length as usize + (BLOCK_SIZE as usize - 1)) / BLOCK_SIZE as usize
}
