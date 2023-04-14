use crate::pod::*;
use crate::reader::ArchiveReader;
use crate::result::*;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::SeekFrom;

const INVALID_GROUP: u64 = 0x7fffffffffffffff;
const EMPTY_GROUP: u64 = 0x0000000000000000;
// const INVALID_DATA: u64 = 0xffffffffffffffff;
const EMPTY_DATA: u64 = 0x8000000000000000;

pub fn is_group(value: u64) -> bool {
    (value & EMPTY_DATA) == 0
}
pub fn is_data(value: u64) -> bool {
    !is_group(value)
}
pub fn is_empty_group(value: u64) -> bool {
    value == EMPTY_GROUP
}
pub fn is_empty_data(value: u64) -> bool {
    value == EMPTY_DATA
}
pub fn address_from_child(child: u64) -> u64 {
    child & INVALID_GROUP
}

#[derive(Debug, Clone)]
pub struct GroupChunk {
    pub position: u64,
    pub child_count: u64, // needs to be a separate variable from the length of the children vec
    pub children: Vec<u64>,
}

impl GroupChunk {
    pub fn load(
        group_pos: u64,
        is_light: bool,
        reader: &mut dyn ArchiveReader,
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
        if child_count > reader.size() / 8 || child_count == 0 {
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
    pub fn is_light(&self) -> bool {
        self.child_count != 0 && self.children.is_empty()
    }

    pub fn load_group(
        &self,
        reader: &mut dyn ArchiveReader,
        index: usize,
        is_light: bool,
    ) -> Result<GroupChunk> {
        if self.is_light() {
            if index < (self.child_count as usize) {
                reader.seek(SeekFrom::Start(self.position + 8 * (index as u64) + 8))?;
                let child_pos = reader.read_u64::<LittleEndian>()?;

                if (child_pos & EMPTY_DATA) == 0 {
                    GroupChunk::load(child_pos, is_light, reader)
                } else {
                    Err(InternalError::DataChunkReadAsGroupChunk.into())
                }
            } else {
                Err(InternalError::OutOfBounds.into())
            }
        } else if is_group(self.children[index]) {
            GroupChunk::load(self.children[index], is_light, reader)
        } else {
            Err(InternalError::DataChunkReadAsGroupChunk.into())
        }
    }

    pub fn load_data(&self, reader: &mut dyn ArchiveReader, index: usize) -> Result<DataChunk> {
        if self.is_light() {
            if index < (self.child_count as usize) {
                reader.seek(SeekFrom::Start(self.position + 8 * (index as u64) + 8))?;
                let child_pos = reader.read_u64::<LittleEndian>()?;
                if (child_pos & EMPTY_DATA) != 0 {
                    DataChunk::load(child_pos, reader)
                } else {
                    Err(InternalError::GroupChunkReadAsDataChunk.into())
                }
            } else {
                Err(InternalError::OutOfBounds.into())
            }
        } else if is_data(self.children[index]) {
            DataChunk::load(self.children[index], reader)
        } else {
            Err(InternalError::GroupChunkReadAsDataChunk.into())
        }
    }
}

#[derive(Debug)]
pub struct DataChunk {
    pub position: u64,
    pub size: u64,
}

impl DataChunk {
    pub fn load(position: u64, reader: &mut dyn ArchiveReader) -> Result<DataChunk> {
        let position = address_from_child(position);

        let size = if position != 0 {
            reader.seek(SeekFrom::Start(position))?;
            // TODO(max): return error if the read size is larger than file size
            reader.read_u64::<LittleEndian>()?
        } else {
            0
        };

        Ok(DataChunk { position, size })
    }

    pub fn read_pod_array(
        &self,
        data_type: &DataType,
        reader: &mut dyn ArchiveReader,
    ) -> Result<PodArray> {
        if self.size < 16 && self.size != 0 {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        const DATA_OFFSET: u64 = 16;

        match data_type.pod_type {
            PodType::String => {
                let char_count = (self.size - 16) as usize;
                let mut char_buffer = vec![0u8; char_count];
                self.read(16, reader, &mut char_buffer)?;

                let mut start_str = 0;
                let mut strings = vec![];
                for i in 0..char_count {
                    if char_buffer[i] == 0 {
                        strings.push(
                            String::from_utf8(char_buffer[start_str..i].to_vec())
                                .map_err(ParsingError::FromUtf8Error)?,
                        );
                        start_str = i + 1;
                    }
                }
                Ok(PodArray::String(strings))
            }
            PodType::WString => todo!(),
            PodType::Boolean => todo!(),
            PodType::U8 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<u8>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_exact(&mut buffer)?;
                Ok(PodArray::U8(buffer))
            }
            PodType::I8 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<i8>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_i8_into(&mut buffer)?;
                Ok(PodArray::I8(buffer))
            }
            PodType::U16 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<u16>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_u16_into::<LittleEndian>(&mut buffer)?;
                Ok(PodArray::U16(buffer))
            }
            PodType::I16 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<i16>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_i16_into::<LittleEndian>(&mut buffer)?;
                Ok(PodArray::I16(buffer))
            }
            PodType::U32 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<u32>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_u32_into::<LittleEndian>(&mut buffer)?;
                Ok(PodArray::U32(buffer))
            }
            PodType::I32 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<i32>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_i32_into::<LittleEndian>(&mut buffer)?;
                Ok(PodArray::I32(buffer))
            }
            PodType::U64 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<u64>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_u64_into::<LittleEndian>(&mut buffer)?;
                Ok(PodArray::U64(buffer))
            }
            PodType::I64 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<i64>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_i64_into::<LittleEndian>(&mut buffer)?;
                Ok(PodArray::I64(buffer))
            }
            PodType::F16 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<f32>();
                let mut buffer = vec![0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_u16_into::<LittleEndian>(&mut buffer)?;
                let buffer = buffer
                    .into_iter()
                    .map(half::f16::from_bits)
                    .collect::<Vec<_>>();
                Ok(PodArray::F16(buffer))
            }
            PodType::F32 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<f32>();
                let mut buffer = vec![0.0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_f32_into::<LittleEndian>(&mut buffer)?;
                Ok(PodArray::F32(buffer))
            }
            PodType::F64 => {
                let element_count = (self.size - DATA_OFFSET) as usize / std::mem::size_of::<f64>();
                let mut buffer = vec![0.0; element_count];
                reader.seek(SeekFrom::Start(self.position + DATA_OFFSET + 8))?;
                reader.read_f64_into::<LittleEndian>(&mut buffer)?;
                Ok(PodArray::F64(buffer))
            }

            PodType::Unknown => Err(UserError::InvalidParameter.into()),
        }
    }

    pub fn read(
        &self,
        offset: u64,
        reader: &mut dyn ArchiveReader,
        buffer: &mut [u8],
    ) -> Result<()> {
        if self.size == 0 || offset + self.size > reader.size() {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        reader.seek(SeekFrom::Start(self.position + offset + 8))?;
        reader.read_exact(buffer)?;

        Ok(())
    }
    pub fn read_u32(&self, offset: u64, reader: &mut dyn ArchiveReader) -> Result<u32> {
        if self.size != 4 {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        reader.seek(SeekFrom::Start(self.position + offset))?;
        let value = reader.read_u32::<LittleEndian>()?;
        Ok(value)
    }
}

pub enum Chunk {
    Group(GroupChunk),
    Data(DataChunk),
}
