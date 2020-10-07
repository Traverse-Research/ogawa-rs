use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::rc::Rc;

use crate::*;

#[derive(Debug, Clone)]
pub(crate) struct GroupChunk {
    pub(crate) position: u64,
    pub(crate) child_count: u64, //needs to be a separate variable from the length of the children vec
    pub(crate) children: Vec<u64>,
}

impl GroupChunk {
    pub(crate) fn load(
        group_pos: u64,
        is_light: bool,
        reader: &mut BufReader<File>,
    ) -> Result<GroupChunk> {
        if is_empty_group(group_pos) {
            return Ok(GroupChunk {
                position: 0,
                child_count: 0,
                children: vec![],
            });
        }

        reader.seek(SeekFrom::Start(group_pos))?;

        let child_count = reader.read_u64::<LittleEndian>()?;
        if child_count > 123456 /*TODO(max): replace with file size / 8?*/|| child_count == 0 {
            return Ok(GroupChunk {
                position: group_pos,
                child_count: 0,
                children: vec![],
            });
        }

        // load child info
        let children = if !is_light || child_count < 9 {
            (0..child_count)
                .map(|_| reader.read_u64::<LittleEndian>().map_err(|err| err.into()))
                .collect::<Result<Vec<_>>>()?
        } else {
            // special case for lights
            vec![]
        };

        Ok(GroupChunk {
            position: group_pos,
            child_count,
            children,
        })
    }
    pub(crate) fn is_child_group(&self, index: usize) -> bool {
        index < self.children.len() && is_group(self.children[index])
    }
    pub(crate) fn is_child_data(&self, index: usize) -> bool {
        index < self.children.len() && is_data(self.children[index])
    }
    pub(crate) fn is_empty_child_group(&self, index: usize) -> bool {
        index < self.children.len() && is_empty_group(self.children[index])
    }
    pub(crate) fn is_empty_child_data(&self, index: usize) -> bool {
        index < self.children.len() && is_empty_data(self.children[index])
    }
    pub(crate) fn is_light(&self) -> bool {
        self.child_count != 0 && self.children.is_empty()
    }

    pub(crate) fn load_group(
        &self,
        reader: &mut BufReader<File>,
        index: usize,
        is_light: bool,
    ) -> Result<GroupChunk> {
        if self.is_light() {
            if index < (self.child_count as usize) {
                reader.seek(SeekFrom::Start(self.position + 8 * (index as u64) + 8))?;
                let child_pos = reader.read_u64::<LittleEndian>()?;

                if (child_pos & EMPTY_DATA) == 0 {
                    Ok(GroupChunk::load(child_pos, is_light, reader)?)
                } else {
                    Err(InternalError::DataChunkReadAsGroupChunk.into())
                }
            } else {
                Err(InternalError::OutOfBounds.into())
            }
        } else if is_group(self.children[index]) {
            Ok(GroupChunk::load(self.children[index], is_light, reader)?)
        } else {
            Err(InternalError::DataChunkReadAsGroupChunk.into())
        }
    }

    pub(crate) fn load_data(
        &self,
        reader: &mut BufReader<File>,
        index: usize,
    ) -> Result<DataChunk> {
        if self.is_light() {
            if index < (self.child_count as usize) {
                reader.seek(SeekFrom::Start(self.position + 8 * (index as u64) + 8))?;
                let child_pos = reader.read_u64::<LittleEndian>()?;
                if (child_pos & EMPTY_DATA) != 0 {
                    Ok(DataChunk::load(child_pos, reader)?)
                } else {
                    Err(InternalError::GroupChunkReadAsDataChunk.into())
                }
            } else {
                Err(InternalError::OutOfBounds.into())
            }
        } else if is_data(self.children[index]) {
            Ok(DataChunk::load(self.children[index], reader)?)
        } else {
            Err(InternalError::GroupChunkReadAsDataChunk.into())
        }
    }
}

#[derive(Debug)]
pub(crate) struct DataChunk {
    pub(crate) position: u64,
    pub(crate) size: u64,
}

impl DataChunk {
    pub(crate) fn load(position: u64, reader: &mut BufReader<File>) -> Result<DataChunk> {
        let position = address_from_child(position);

        let size = if position != 0 {
            reader.seek(SeekFrom::Start(position))?;
            let size = reader.read_u64::<LittleEndian>()?;
            //TODO(max): return error if read size is larger than file size
            size
        } else {
            0
        };

        Ok(DataChunk { position, size })
    }

    pub(crate) fn read(
        &self,
        offset: u64,
        reader: &mut BufReader<File>,
        buffer: &mut [u8],
    ) -> Result<()> {
        if self.size == 0
        /* || offset + size > file_size*/
        {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        reader.seek(SeekFrom::Start(self.position + offset + 8))?;
        reader.read(buffer)?;

        Ok(())
    }
    pub(crate) fn read_u32(&self, offset: u64, reader: &mut BufReader<File>) -> Result<u32> {
        if self.size != 4 {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        reader.seek(SeekFrom::Start(self.position + offset))?;
        let value = reader.read_u32::<LittleEndian>()?;
        Ok(value)
    }
}

pub(crate) enum Chunk {
    Group(GroupChunk),
    Data(DataChunk),
}
