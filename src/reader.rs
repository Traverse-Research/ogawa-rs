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

pub trait ArchiveReader: std::io::Read + std::io::Seek {
    fn size(&self) -> u64;
}

pub struct MemMappedReader {
    _file: File,
    cursor: std::io::Cursor<memmap::Mmap>,
    size: u64,
}

impl std::io::Read for MemMappedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(self.cursor.read(buf).unwrap())
    }
}
impl std::io::Seek for MemMappedReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        Ok(self.cursor.seek(pos).unwrap())
    }
}
impl ArchiveReader for MemMappedReader {
    fn size(&self) -> u64 {
        self.size
    }
}

impl MemMappedReader {
    pub fn new(mut file: std::fs::File) -> Result<Self> {
        let old_pos = file.seek(SeekFrom::Current(0))?;
        let size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(old_pos))?;

        let mmap = unsafe { memmap::Mmap::map(&file) }?;

        let cursor = std::io::Cursor::new(mmap);

        Ok(Self {
            _file: file,
            cursor,
            size,
        })
    }
}

pub struct FileReader {
    pub file: BufReader<File>,
    pub size: u64,
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
impl ArchiveReader for FileReader {
    fn size(&self) -> u64 {
        self.size
    }
}

impl FileReader {
    pub fn new(file: File) -> Result<FileReader> {
        let mut file = BufReader::new(file);

        let size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;

        Ok(FileReader { file, size })
    }
}
