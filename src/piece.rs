use std::{
    collections::{btree_map::Entry, BTreeMap},
    ops::Range,
    sync::RwLock,
};

use sha1::{Digest, Sha1};

use crate::{
    block::{Block, BlockInfo, BLOCK_SIZE},
    error::TorrusError,
    metainfo::Metainfo,
    storage::TorrentFile,
    utils::RangeExt,
    Bitfield, Hash, PieceIndex, Result,
};

/// Tracks all the pieces in the current torrent.
pub struct PieceHandler {
    /// bitfield tracks the pieces which the client has
    bitfield: Bitfield,
    /// Pre allocated to the number of pieces in the torrent
    pieces: Vec<Piece>,
    /// Total number of missing pieces
    miss_count: usize,
    /// Total have count
    have_count: usize,
    /// files in the torrent
    files: Vec<RwLock<TorrentFile>>,
}

#[derive(Clone, Default)]
pub struct PieceInfo {
    /// The index of the piece in the bitfield
    pub index: PieceIndex,
    /// The frequency of the piece in the swarm.
    pub frequency: usize,
    /// The piece is pending or not
    pub pending: bool,
    /// 20 byte Sha-1 Hash of the piece
    pub hash: Hash,
    /// legth of the piece
    pub len: usize,
    /// Offset of the piece within the torrent
    pub offset: usize,
    /// Range of file indexes which overlaps with the piece
    pub file_range: Range<usize>,
}

/// Represents an individual piece in a torrent.
#[derive(Clone, Default)]
pub struct Piece {
    /// information regarding the piece
    piece_info: PieceInfo,
    /// THe blocks of the piece
    pub blocks: BTreeMap<u32, Block>,
}

impl Piece {
    pub fn new(piece_info: PieceInfo) -> Self {
        let blocks = BTreeMap::new();

        Self { piece_info, blocks }
    }

    pub fn validate(&self) -> bool {
        let mut hasher = Sha1::new();

        for block in self.blocks.values() {
            hasher.update(&block.data);
        }

        let hash = hasher.finalize();

        log::debug!("Piece Hash: {:x}", hash);

        hash.as_slice() == self.hash()
    }

    pub fn is_complete(&self) -> bool {
        self.blocks.len() == crate::block::block_count(self.length() as u64)
    }

    pub fn insert_block(&mut self, block: Block) -> Result<()> {
        let index = block.block_info.begin;

        let entry = self.blocks.entry(index);

        use Entry::*;
        match entry {
            Vacant(_) => {
                self.blocks.insert(index, block);
                Ok(())
            }

            Occupied(_) => Err(TorrusError::new("Duplicate Block")),
        }
    }

    /// Iterate over all the files for which the [`Piece`] overlaps with, find all the [`Block`]
    /// which overlap over a certain file and then finally write the data to the [`TorrentFile`]
    pub fn write(&self, files: &mut [RwLock<TorrentFile>]) -> Result<()> {
        let files = &mut files[self.piece_info.file_range.clone()];

        let _res: Vec<_> = files
            .iter_mut()
            .map(|file| -> Vec<Result<()>> {
                let mut file = file.write().unwrap();

                self.blocks
                    .values()
                    .map(|block| -> Result<()> {
                        let byte_range = block.byte_range();

                        let file_byte_range = file.byte_range();

                        let intersection =
                            RangeExt::new(vec![byte_range, file_byte_range]).intersection();

                        let data = block.get_slice(intersection);

                        file.write(data)?;

                        Ok(())
                    })
                    .collect()
            })
            .collect();

        Ok(())
    }

    pub fn length(&self) -> usize {
        self.piece_info.len
    }

    pub fn hash(&self) -> &Hash {
        &self.piece_info.hash
    }

    pub fn request_block(&self) -> Option<BlockInfo> {
        for (_, block) in self.blocks.iter() {
            if block.len() < (BLOCK_SIZE as usize) {
                return Some(block.block_info);
            }
        }
        None
    }
}

impl PieceHandler {
    pub fn from_metainfo(
        metainfo: &Metainfo,
        bitfield: Bitfield,
        files: Vec<RwLock<TorrentFile>>,
    ) -> Self {
        let piece_length = metainfo.info.piece_length;

        let pieces_hash = metainfo.info.pieces.clone();

        let mut pieces = Vec::new();

        let pieces_range = 0..metainfo.total_pieces();

        let hash_range = (0..pieces_hash.len()).step_by(20);

        let mut piece_offset = 0;

        for (piece_index, offset) in pieces_range.zip(hash_range) {
            let mut piece_info= PieceInfo::default();

            piece_info.len = piece_length as usize;
            piece_info.hash = pieces_hash[offset..(offset + 20)].to_vec();
            piece_info.index = piece_index;
            piece_info.pending = true;
            piece_info.offset = piece_offset;

            piece_offset += piece_info.len;

            let files_overlap = get_overlapping_range(&files, &piece_info);

            piece_info.file_range = files_overlap;

            let piece = Piece::new(piece_info);

            pieces.push(piece);
        }

        let miss_count = bitfield.count_zeros();
        let have_count = bitfield.count_ones();

        Self {
            bitfield,
            pieces,
            miss_count,
            have_count,
            files,
        }
    }

    pub fn get_bitfield(&self) -> &Bitfield {
        &self.bitfield
    }

    pub fn miss_count(&self) -> usize {
        self.miss_count
    }

    pub fn have_count(&self) -> usize {
        self.have_count
    }

    /// For now the [`PieceHandler`] picks a [`Piece`] which is pending later a rarest first
    /// algorithm should be implemented.
    pub fn pick_piece(&self, index: &PieceIndex) -> BlockInfo {
        let piece = &self.pieces[*index];

        log::debug!("\tpick_piece:\tpicked{}", piece.piece_info.index);

        match piece.request_block() {
            Some(block_info) => block_info,
            None => BlockInfo {
                piece_index: piece.piece_info.index,
                begin: 0,
                length: BLOCK_SIZE,
            },
        }
    }

    /// Insert a [`Block`] into the coresponding [`Piece`].
    /// If the Piece is complete then write it to the disk.
    pub fn insert_block(&mut self, block: Block) -> Result<()> {
        let index = block.block_info.piece_index;
        let piece = &mut self.pieces[index];

        piece.insert_block(block)?;

        if piece.is_complete() && piece.validate() {
            piece.write(&mut self.files)?;
        }

        Ok(())
    }

    /// Check if the input [`Bitfield`] matches with [`PieceHandler`]'s Bitfield
    pub fn match_bitfield_len(&self, len: usize) -> bool {
        self.bitfield.capacity() == len
    }
}

/// Finds the overlapping [`TorrentFile`] given a individual [`PieceInfo`]. Bittorrent specs
/// specify that a Torrent can be thought of as a large contineous byte array, so a [`Piece`] might
/// overlap with multiple files in the said byte array.
fn get_overlapping_range(files: &[RwLock<TorrentFile>], piece: &PieceInfo) -> Range<usize> {
    let piece_range = piece.offset..(piece.offset + piece.len);

    let first_index = match files
        .iter()
        .enumerate()
        .find(|(_, file)| {
            let file = file.read().unwrap();

            file.byte_range().contains(&(piece_range.start))
        })
        .map(|(index, _)| index)
    {
        Some(index) => index,
        None => return 0..0,
    };

    let mut file_range = first_index..first_index + 1;
    for (index, file) in files.iter().enumerate().skip(first_index + 1) {
        let file = file.read().unwrap();

        if !piece_range.contains(&(file.get_offset())) {
            break;
        }

        file_range.end = index + 1;
    }

    file_range
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rand::Rng;

    use super::*;
    use crate::{block::BlockInfo, storage::FileInfo, Result};

    #[test]
    fn test_validate() -> Result<()> {
        let data = "Hello".as_bytes();

        let mut hasher = Sha1::new();

        hasher.update(data);

        let block_info = BlockInfo {
            begin: 0,
            piece_index: 0,
            length: 0,
        };

        let block = Block::new(block_info, data);

        let mut piece_info = PieceInfo::default();
        piece_info.hash = hasher.finalize().to_vec();

        let mut piece = Piece::new(piece_info);

        piece.insert_block(block)?;

        assert!(piece.validate());
        Ok(())
    }

    #[test]
    fn test_piece_overlap() -> Result<()> {
        let data = rand::thread_rng().gen::<[u8; 20]>();
        let mut hasher = Sha1::new();
        hasher.update(data);

        let hash = hasher.finalize();

        let mut files = Vec::new();
        for i in 0..=20 {
            let path = PathBuf::from(format!("/tmp/file{}.txt", i));
            let file_info = FileInfo {
                path,
                length: 1,
                offset: i,
            };

            let file = RwLock::new(TorrentFile::new(file_info)?);

            files.push(file);
        }
        let mut piece_info = PieceInfo::default();

        piece_info.hash = hash.to_vec();
        piece_info.len = data.len();

        let mut piece = Piece::new(piece_info);

        let block_info = BlockInfo {
            piece_index: 0,
            begin: 0,
            length: 0,
        };

        let block = Block::new(block_info, data);

        piece.insert_block(block)?;

        let range = get_overlapping_range(&files, &piece.piece_info);

        let expected_range = Range { start: 0, end: 20 };
        assert_eq!(range, expected_range);
        Ok(())
    }

    #[test]
    fn test_piece() -> Result<()> {
        let mut files = Vec::new();
        for i in 0..21 {
            let path = PathBuf::from(format!("/tmp/file{}.txt", i));
            let file_info = FileInfo {
                path,
                length: 1,
                offset: i,
            };

            let file = RwLock::new(TorrentFile::new(file_info)?);

            files.push(file);
        }

        let loop_range = 0..=3;
        let file_range = (0..21).step_by(7);

        for (i, j) in loop_range.zip(file_range) {
            let mut piece_info = PieceInfo::default();

            piece_info.offset = 7 * i;
            piece_info.len = 7;
            piece_info.index = i;

            let piece = Piece::new(piece_info);

            let range = get_overlapping_range(&files, &piece.piece_info);

            let expected_range = Range {
                start: j,
                end: j + 7,
            };

            println!("Start: {}, End: {}", range.start, range.end);
            assert_eq!(range, expected_range);
        }

        Ok(())
    }
}
