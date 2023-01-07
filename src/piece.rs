use std::collections::BTreeMap;

use sha1::{Digest, Sha1};

use crate::{block::Block, Bitfield, Hash, PieceIndex};

/// Tracks all the pieces in the current torrent.
pub struct PieceTracker {
    /// bitfield tracks the pieces which the client has
    bitfield: Bitfield,
    /// Pre allocated to the number of pieces in the torrent
    pieces: Vec<Piece>,
    /// Total number of missing pieces
    miss_count: usize,
    /// Total have count
    have_count: usize,
}

/// Represents an individual piece in a torrent.
#[derive(Clone, Default)]
struct Piece {
    /// The index of the piece in the bitfield
    pub index: PieceIndex,
    /// The frequency of the piece in the swarm.
    pub frequency: usize,
    /// The piece is pending or not
    pub pending: bool,
    /// 20 byte Sha-1 Hash of the piece
    pub hash: Hash,
    /// THe blocks of the piece
    pub blocks: BTreeMap<u32, Block>,
    /// legth of the piece
    pub len: u32,
}

impl Piece {
    pub fn validate(&self) -> bool {
        let mut hasher = Sha1::new();

        for block in self.blocks.values() {
            hasher.update(&block.data);
        }

        let hash = hasher.finalize();

        log::debug!("Piece Hash: {:x}", hash);

        hash.as_slice() == self.hash
    }

    pub fn is_complete(&self) -> bool {
        self.blocks.len() == crate::block::block_count(self.len)
    }
}

impl PieceTracker {
    pub fn new(bitfield: Bitfield) -> Self {
        let pieces = Vec::new();
        let miss_count = bitfield.count_zeros();
        let have_count = bitfield.count_ones();

        Self {
            bitfield,
            pieces,
            miss_count,
            have_count,
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

            if !self.bitfield[index] && piece.frequency > 0 && piece.pending {
                return Some(index);
            }
        }

        return None;
    }
}
