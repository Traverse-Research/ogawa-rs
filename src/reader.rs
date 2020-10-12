use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;

use crate::result::{ParsingError, Result};

pub(crate) trait StringReader {
    fn read_string(&mut self, size: usize) -> Result<String>;
}
impl StringReader for std::io::Cursor<Vec<u8>> {
    fn read_string(&mut self, size: usize) -> Result<String> {
        let mut buffer = vec![0u8; size];
        self.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer).map_err(ParsingError::FromUtf8Error)?)
    }
}

pub trait ArchiveReader: std::io::Read + std::io::Seek {}

pub struct MemMappedReader {
    _file: File,
    mmap: memmap::Mmap,
    position: u64,
    size: u64,
}

impl std::io::Read for MemMappedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = self.position as usize;
        let end = self.position as usize + buf.len();

        if end as u64 > self.size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                anyhow::anyhow!("failed to fill whole buffer"),
            ));
        }

        buf.copy_from_slice(&self.mmap[start..end]);

        self.position = end as u64;

        Ok(end - start)
    }
}
impl std::io::Seek for MemMappedReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Current(x) => self.position as i128 + x as i128,
            SeekFrom::Start(x) => x as i128,
            SeekFrom::End(x) => (self.size as i128 - x as i128) as i128,
        };

        if new_pos < 0 {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                anyhow::anyhow!("attempted to seek to negative position"),
            ))
        } else if new_pos as u64 > self.size {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                anyhow::anyhow!("attempted to seek past end of resource"),
            ))
        } else {
            self.position = new_pos as u64;
            Ok(self.position)
        }
    }
}
impl ArchiveReader for MemMappedReader {}

impl MemMappedReader {
    pub fn new(mut file: std::fs::File) -> Result<Self> {
        let old_pos = file.seek(SeekFrom::Current(0))?;
        let size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(old_pos))?;

        let mmap = unsafe { memmap::Mmap::map(&file) }?;

        Ok(Self {
            _file: file,
            mmap,
            position: 0,
            size,
        })
    }
}

pub struct FileReader {
    pub file: BufReader<File>,
    pub file_size: u64,
}

impl std::io::Read for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}
impl std::io::Seek for FileReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}
impl ArchiveReader for FileReader {}

impl FileReader {
    pub fn new(file: File) -> Result<FileReader> {
        let mut file = BufReader::new(file);

        let file_size = file.seek(SeekFrom::End(0))?;
        let _ = file.seek(SeekFrom::Start(0));

        Ok(FileReader { file, file_size })
    }
}
