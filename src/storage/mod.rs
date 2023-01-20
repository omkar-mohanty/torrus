use crate::error::TorrusError;
use crate::{Offset, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::{Deref, Range};
use std::path::PathBuf;

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
    /// Download directory of the torrent
    pub download_dir: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub offset: u64,
    pub length: u64,
}

pub struct TorrentFile {
    file_info: FileInfo,
    file: File,
}

impl TorrentFile {
    pub fn new(file_info: FileInfo) -> Result<Self> {
        let path = &file_info.path;

        let file = if path.exists() {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .map_err(|e| {
                    log::warn!("Failed to open file {:?}", path);
                    TorrusError::new(&e.to_string())
                })?
        } else {
            File::options()
                .write(true)
                .read(true)
                .create(true)
                .open(path)
                .map_err(|e| {
                    log::warn!("Failed to open file {:?}", path);
                    TorrusError::new(&e.to_string())
                })?
        };

        Ok(Self { file_info, file })
    }

    pub fn write(&mut self, data: IoVec) -> Result<()> {
        self.file.seek(SeekFrom::Start(data.begin as u64))?;

        self.file.write_all(&data)?;
        Ok(())
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // for some reason self.file.read(&mut buf) does not read anything.
        let mut file = File::open(&self.file_info.path)?;

        let len = file.read(buf)?;

        log::info!("Read : {}", len);

        Ok(len)
    }

    pub fn from_metainfo(metainfo_file: crate::metainfo::File, offset: u64) -> Result<Self> {
        let path: PathBuf = metainfo_file.path.iter().collect();
        let length = metainfo_file.length;
        let file_info = FileInfo {
            path,
            length,
            offset,
        };

        Self::new(file_info)
    }

    pub fn get_offset(&self) -> Offset {
        self.file_info.offset as Offset
    }

    pub fn get_length(&self) -> Offset {
        self.file_info.length as Offset
    }

    pub fn byte_range(&self) -> Range<Offset> {
        let offset = self.get_offset();
        let end = offset + self.get_length();
        offset..end
    }
}

#[derive(Debug, Clone)]
pub struct IoVec {
    /// Byte offset from where the data actually begins
    pub begin: Offset,
    /// Actual data in bytes
    data: Vec<u8>,
}

impl IoVec {
    pub fn new(begin: Offset, data: Vec<u8>) -> Self {
        IoVec { begin, data }
    }
}

impl Deref for IoVec {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metainfo::Metainfo;
    use crate::Result;
    use std::fs;
    use std::io::Read;
    use std::path::Path;

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
        let mut paths = vec![];
        let file_infos: Vec<FileInfo> = {
            let mut res = vec![];

            let files = metainfo.info.files.unwrap();
            let mut offset: u64 = 0;
            for file in files {
                let file_path: PathBuf = file.path.iter().collect();

                let path = PathBuf::from("/tmp");

                let path = path.join(file_path);

                paths.push(path.clone());

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

        for path in paths {
            assert!(path.exists());
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_file_write() -> Result<()> {
        let path = PathBuf::from("/tmp/write.txt");

        let mut torrent_file = {
            let file_info = FileInfo {
                path: path.clone(),
                offset: 0,
                length: 0,
            };

            TorrentFile::new(file_info)?
        };

        let data = "Hello".as_bytes().to_vec();

        let io_vec = IoVec::new(0, data);

        torrent_file.write(io_vec)?;

        let mut file = File::open(path)?;

        let mut contents = String::new();

        file.read_to_string(&mut contents)?;

        assert_eq!(&contents, "Hello");
        Ok(())
    }

    #[test]
    fn test_file_read() -> Result<()> {
        let path = PathBuf::from("/tmp/read.txt");

        let file_info = FileInfo {
            length: "Hello".len() as u64,
            offset: 0,
            path: path.clone(),
        };

        let mut torrent_file = TorrentFile::new(file_info)?;

        torrent_file.write(IoVec::new(0, "Hello".as_bytes().to_vec()))?;

        let mut buf1: [u8; 1024] = [0; 1024];

        File::open(path.clone())?.read(&mut buf1)?;

        let mut buf2: [u8; 1024] = [0; 1024];

        torrent_file.read(&mut buf2)?;

        assert_eq!(buf1, buf2);
        Ok(())
    }
}
