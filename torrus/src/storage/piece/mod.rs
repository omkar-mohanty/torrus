mod manager;

pub use manager::PieceManager;

use super::{Block, Blockinfo};
use crate::Result;
use std::{io::prelude::*, io::Cursor, io::SeekFrom};

const BLOCK_SIZE: u64 = 2 << 14;

pub struct PieceInfo {
    length: u64,
    hash: Vec<u8>,
    piece_index: usize,
}

/// A [Piece] should be able to :-
///
/// 1. Verify it's integrity
/// 2. Rject a [Block] which does not match it's current byte offset
///
/// It does not currently however check if it is completed, that is the job of the caller
pub struct Piece {
    piece_info: PieceInfo,
    data: Vec<u8>,
    byte_index: u64,
}

impl Piece {
    pub fn new(piece_info: PieceInfo) -> Self {
        Self {
            piece_info,
            data: Vec::new(),
            byte_index: 0,
        }
    }

    pub fn is_full(&self) -> bool {
        self.size() == self.piece_info.length
    }

    pub fn size(&self) -> u64 {
        self.byte_index
    }

    /// For simplicity [write] only accepts data whose offset is exactly the current index.
    pub fn write(&mut self, block: Block) -> Result<()> {
        assert!(self.byte_index + block.block_info.length <= self.piece_info.length);
        assert_eq!(block.block_info.offset, self.byte_index);
        let mut cursor = Cursor::new(&mut self.data);
        cursor.seek(SeekFrom::Start(self.byte_index))?;
        cursor.write(&block)?;
        self.byte_index = self.byte_index + (block.len() as u64);
        Ok(())
    }

    fn get_raw_data(self) -> Vec<u8> {
        self.data
    }

    fn check_integrity(&self) -> bool {
        todo!()
    }

    pub fn next_block(&self) -> Blockinfo {
        let required_length = self.piece_info.length - self.byte_index;

        let mut length = BLOCK_SIZE;

        if required_length < BLOCK_SIZE {
            length = required_length;
        }

        Blockinfo {
            offset: self.byte_index,
            index: self.piece_info.piece_index,
            length,
        }
    }

    pub fn can_be_flushed(&self) -> bool {
        if !self.is_full() {
            return false;
        }

        if !self.check_integrity() {
            return false;
        }

        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_block(index: usize, length: u64, offset: u64) -> Block {
        let block_info = Blockinfo {
            offset,
            length,
            index,
        };
        Block::new(&vec![10; length as usize], block_info)
    }

    #[test]
    fn test_piece_write() -> crate::Result<()> {
        let piece_info = PieceInfo {
            piece_index: 0,
            hash: vec![],
            length: 20,
        };
        let block = get_test_block(0, 10, 0);

        let mut piece = Piece::new(piece_info);
        piece.write(block)?;

        let block = get_test_block(0, 10, 10);

        piece.write(block)?;

        assert_eq!(vec![10; 20], piece.get_raw_data());
        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_wrong_offset_piece_write() {
        let piece_info = PieceInfo {
            piece_index: 0,
            hash: vec![],
            length: 10,
        };

        let block = get_test_block(0, 10, 3);

        let mut piece = Piece::new(piece_info);
        piece.write(block).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_piece_overwrite() {
        let piece_info = PieceInfo {
            piece_index: 0,
            hash: vec![],
            length: 10,
        };

        let block = get_test_block(0, 10, 0);

        let mut piece = Piece::new(piece_info);
        piece.write(block).unwrap();

        let block = get_test_block(0, 10, 10);

        piece.write(block).unwrap();

        assert_eq!(vec![10; 20], piece.get_raw_data());
    }

    #[test]
    fn test_next_block_request_less_than_block_size() -> crate::Result<()> {
        let piece_info = PieceInfo {
            piece_index: 0,
            hash: vec![],
            length: 10,
        };
        let mut piece = Piece::new(piece_info);
        for index in 0..=10 {
            let block = get_test_block(0, 1, index);

            piece.write(block)?;

            let next_block = piece.next_block();

            assert_eq!(next_block.offset, index + 1);
        }
        Ok(())
    }
}
