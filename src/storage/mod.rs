use std::fs::File;
use std::{fs, path::PathBuf};

use crate::Result;

#[derive(Clone, Debug)]
pub struct DiskInfo {
    /// The number of pieces in the torrent
    pub piece_count: u32,
    /// The length of a piece
    pub piece_len: u32,
    /// The length of the last piece if size of the torrent is not a multiple of piece_length
    pub last_piece_length: u32,
    /// Meta Info of all the files in the torrent
    pub files: Vec<FileInfo>,
}

#[derive(Clone, Debug)]
pub struct FileInfo {
    path: PathBuf,
    offset: u64,
    length: u64,
}

pub struct TorrentFile {
    file_info: FileInfo,
    file: File,
}

impl TorrentFile {
    fn new(file_info: FileInfo) -> Result<Self> {
        let path = &file_info.path;

        let file = fs::File::create(path)?;

        Ok(Self { file_info, file })
    }
}
#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::metainfo::Metainfo;
    use crate::Result;

    const PATH_SINGLE: &str = "./resources/ubuntu-22.10-desktop-amd64.iso.torrent";
    const PATH_MULTI: &str = "./resources/multi.torrent";

    #[tokio::test]
    async fn test_file_create() -> Result<()> {
        let file_path = "/tmp/torrustest.tst";

        let path = PathBuf::from(file_path);

        let file_info = FileInfo {
            path,
            offset: 0,
            length: 0,
        };

        let _ = TorrentFile::new(file_info)?;

        assert!(Path::new(file_path).exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_file_create_metainfo() -> Result<()> {
        let file = fs::read(PATH_SINGLE)?;
        let metainfo = Metainfo::from_bytes(&file).unwrap();

        let path: PathBuf = metainfo.info.name.into();

        let file_info = FileInfo {
            path: path.clone(),
            offset: 0,
            length: metainfo.info.length,
        };

        let _ = TorrentFile::new(file_info);

        assert!(Path::new(&path).exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_multi() -> Result<()> {
        let file = fs::read(PATH_MULTI)?;
        let metainfo = Metainfo::from_bytes(&file).unwrap();

        let file_infos: Vec<FileInfo> = {
            let mut res = vec![];

            let files = metainfo.info.files.unwrap();
            let mut offset: u64 = 0;
            for file in files {
                let path: PathBuf = file.path.iter().collect();

                res.push(FileInfo {
                    path,
                    offset,
                    length: file.length,
                });
                offset += file.length;
            }

            res
        };

        for info in file_infos {
            TorrentFile::new(info)?;
        }
        Ok(())
    }
}
