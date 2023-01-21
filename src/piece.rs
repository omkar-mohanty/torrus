use std::{
    collections::{btree_map::Entry, BTreeMap},
    ops::Range,
    sync::RwLock,
};

use sha1::{Digest, Sha1};

use crate::{
    block::Block,
    metainfo::Metainfo,
    storage::{IoVec, TorrentFile},
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

    pub fn pending(&self) -> bool {
        self.piece_info.pending
    }

    pub fn frequency(&self) -> usize {
        self.piece_info.frequency
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
        let res = match entry {
            Vacant(_) => {
                self.blocks.insert(index, block);
                Ok(())
            }

            Occupied(_) => Err("Duplicate Block".into()),
        };

        res
    }

    pub fn write(&self, files: &mut [RwLock<TorrentFile>]) -> Result<()> {
        let files = &mut files[self.piece_info.file_range.clone()];

        let _res: Vec<_> = files
            .iter_mut()
            .map(|file| -> Vec<Result<()>> {
                let mut file = file.write().unwrap();

                self.blocks
                    .iter()
                    .map(|(_, block)| -> Result<()> {
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

    pub fn offset(&self) -> usize {
        self.piece_info.offset
    }

    pub fn length(&self) -> usize {
        self.piece_info.len
    }

    pub fn hash(&self) -> &Hash {
        &self.piece_info.hash
    }

    /// Assumes the files array is sorted accodring to offset otherwise the output range may not be
    /// contineous
    pub fn get_overlapping_files(&self) -> Range<usize> {
        self.piece_info.file_range.clone()
    }

    fn get_data(&self, file_offset: u64) -> IoVec {
        unimplemented!()
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
            let mut piece_info = PieceInfo::default();

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

    pub fn miss_count(&self) -> usize {
        self.miss_count
    }

    pub fn have_count(&self) -> usize {
        self.have_count
    }

    pub fn pick_piece(&self) -> Option<PieceIndex> {
        for index in 0..self.bitfield.len() {
            let piece = &self.pieces[index];

            if !self.bitfield[index] && piece.frequency() > 0 && piece.pending() {
                return Some(index);
            }
        }

        return None;
    }

    pub fn insert_block(&mut self, block: Block) -> Result<()> {
        let index = block.block_info.piece_index;

        Ok(self.pieces[index].insert_block(block)?)
    }
}

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

        if !piece_range.contains(&(file.get_offset() as usize)) {
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
