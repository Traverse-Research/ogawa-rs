use super::{PropertyHeader, PropertyReader};
use crate::chunks::*;
use crate::pod::*;
use crate::reader::ArchiveReader;
use crate::result::*;

pub use std::convert::TryInto;

#[derive(Debug)]
pub struct ArrayPropertyReader {
    pub group: GroupChunk,
    pub header: PropertyHeader,
}

impl ArrayPropertyReader {
    pub fn new(group: GroupChunk, header: PropertyHeader) -> Self {
        Self { group, header }
    }
    pub fn name(&self) -> &str {
        &self.header.name
    }

    pub fn is_constant(&self) -> bool {
        self.header.first_changed_index == 0
    }
    pub fn sample_count(&self) -> u32 {
        self.header.next_sample_index
    }
    pub fn load_sample(&self, index: u32, reader: &mut dyn ArchiveReader) -> Result<PodArray> {
        if index >= self.header.next_sample_index {
            return Err(UserError::OutOfBounds.into());
        }

        let index = self.header.map_index(index);
        let data = self.group.load_data(reader, index)?;
        data.read_pod_array(&self.header.data_type, reader)
    }
    pub fn sample_size(&self, index: u32, reader: &mut dyn ArchiveReader) -> Result<usize> {
        if index >= self.header.next_sample_index {
            return Err(UserError::OutOfBounds.into());
        }

        let index = self.header.map_index(index);
        let data = self.group.load_data(reader, index)?;
        Ok(data.size as usize)
    }
}

impl std::convert::TryFrom<PropertyReader> for ArrayPropertyReader {
    type Error = ParsingError;
    fn try_from(reader: PropertyReader) -> Result<Self, Self::Error> {
        if let PropertyReader::Array(r) = reader {
            Ok(r)
        } else {
            Err(ParsingError::IncompatibleSchema)
        }
    }
}
