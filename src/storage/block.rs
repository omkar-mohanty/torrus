use core::ops::Deref;

pub struct Blockinfo {
    pub offset: u64,
    pub length: u64,
    pub index: usize,
}

/// Block is basic unit of all operations in storage module. No modules outside storage should be
/// aware of anything other than [Block] and TorrentEngine.
pub struct Block {
    pub block_info: Blockinfo,
    data: Vec<u8>,
}

impl Block {
    pub fn new(data: &[u8], block_info: Blockinfo) -> Self {
        Self {
            block_info,
            data: data.to_vec(),
        }
    }
}

impl Deref for Block {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
