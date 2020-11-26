use std::rc::Rc;

use byteorder::{LittleEndian, ReadBytesExt};

mod chunks;
mod metadata;
mod object_reader;
mod pod;
mod property;
mod reader;
mod result;
mod schemas;
mod time_sampling;

pub use chunks::*;
use metadata::MetaData;
pub use object_reader::{ObjectHeader, ObjectReader};
pub use pod::*;
pub use property::*;
pub use reader::{ArchiveReader, FileReader, MemMappedReader};
pub use result::{InternalError, OgawaError, ParsingError, Result, UserError};
pub use schemas::{CurvesSchema, Schema};
pub use time_sampling::{TimeSampling, TimeSamplingType};

pub struct Archive {
    pub alembic_file_version: u16,
    pub version: u32,
    pub ogawa_file_version: u32,

    pub root_group: GroupChunk,
    pub root_header: ObjectHeader,

    pub time_samplings: Vec<Rc<TimeSampling>>,
    pub max_samples: Vec<i64>,
    pub indexed_meta_data: Vec<MetaData>,
}

impl Archive {
    pub fn new(reader: &mut dyn ArchiveReader) -> Result<Self> {
        let mut magic = vec![0; 5];
        reader.read_exact(&mut magic)?;

        if magic != [0x4f, 0x67, 0x61, 0x77, 0x61] {
            return Err(ParsingError::UnsupportedAlembicFile.into());
        }

        let _frozen = reader.read_u8()? == 0xff;
        let alembic_file_version = reader.read_u16::<LittleEndian>()?;
        if alembic_file_version >= 9999 {
            return Err(ParsingError::UnsupportedAlembicFile.into());
        }
        let group_pos = reader.read_u64::<LittleEndian>()?;

        let root_group = GroupChunk::load(group_pos, false, reader)?;

        if root_group.child_count <= 5
            || !is_data(root_group.children[0] /*  version */)
            || !is_data(root_group.children[1] /*  file version */)
            || !is_group(root_group.children[2] /* root? */)
            || !is_data(root_group.children[3] /*  metadata */)
            || !is_data(root_group.children[4] /*  time sampling */)
            || !is_data(root_group.children[5] /*  indexed metadata */)
        {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        let version = {
            let data = root_group.load_data(reader, 0)?;
            data.read_u32(0, reader)?
        };

        let ogawa_file_version = {
            let data = root_group.load_data(reader, 1)?;
            data.read_u32(0, reader)?
        };

        let meta_data = {
            let data = root_group.load_data(reader, 3)?;
            let mut buffer = vec![0u8; data.size as usize];
            data.read(0, reader, &mut buffer)?;
            let text = String::from_utf8(buffer).map_err(ParsingError::FromUtf8Error)?;

            MetaData::deserialize(&text)
        };

        let (time_samplings, max_samples) = {
            let data = root_group.load_data(reader, 4)?;
            time_sampling::read_time_samplings_and_max(&data, reader)?
        };

        let indexed_meta_data = {
            let data = root_group.load_data(reader, 5)?;
            metadata::read_indexed_meta_data(&data, reader)?
        };

        let root_header = ObjectHeader {
            name: "ABC".to_owned(),
            full_name: "/".to_owned(),
            meta_data,
        };

        Ok(Archive {
            alembic_file_version,
            version,
            ogawa_file_version,

            root_group,
            root_header,

            time_samplings,
            max_samples,
            indexed_meta_data,
        })
    }

    pub fn load_root_object(&self, reader: &mut dyn ArchiveReader) -> Result<ObjectReader> {
        let group = self.root_group.load_group(reader, 2, false)?;
        ObjectReader::new(
            group,
            "",
            reader,
            &self.indexed_meta_data,
            &self.time_samplings,
            self.root_header.clone(),
        )
    }
}
