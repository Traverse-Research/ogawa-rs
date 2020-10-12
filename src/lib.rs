use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::rc::Rc;

mod object_reader;
pub use object_reader::{ObjectHeader, ObjectReader};
mod property;
pub use property::*;
mod chunks;
pub use chunks::*;

mod pod;
pub use pod::*;

mod result;
pub use result::{InternalError, OgawaError, ParsingError, Result, UserError};

mod metadata;
use metadata::MetaData;

mod time_sampling;
pub use time_sampling::{TimeSampling, TimeSamplingType};

trait StringReader {
    fn read_string(&mut self, size: usize) -> Result<String>;
}
impl StringReader for std::io::Cursor<Vec<u8>> {
    fn read_string(&mut self, size: usize) -> Result<String> {
        let mut buffer = vec![0u8; size];
        self.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer).map_err(ParsingError::FromUtf8Error)?)
    }
}

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

pub struct FileReader {
    pub file: BufReader<File>,
    pub file_size: u64,
}
impl FileReader {
    pub fn new(file_name: &str) -> Result<FileReader> {
        let file = File::open(file_name)?;
        let mut file = BufReader::new(file);

        let file_size = file.seek(SeekFrom::End(0))?;
        let _ = file.seek(SeekFrom::Start(0));

        Ok(FileReader { file, file_size })
    }
}

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
    pub fn new(reader: &mut FileReader) -> Result<Self> {
        let mut magic = vec![0; 5];
        reader.file.read_exact(&mut magic)?;
        if magic != [0x4f, 0x67, 0x61, 0x77, 0x61] {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        let _frozen = reader.file.read_u8()? == 0xff;
        let alembic_file_version = reader.file.read_u16::<LittleEndian>()?;
        if alembic_file_version >= 9999 {
            return Err(ParsingError::UnsupportedAlembicFile.into());
        }
        let group_pos = reader.file.read_u64::<LittleEndian>()?;

        let root_group = GroupChunk::load(group_pos, false, &mut reader.file)?;

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
            let data = root_group.load_data(&mut reader.file, 0)?;
            data.read_u32(0, &mut reader.file)?
        };

        let ogawa_file_version = {
            let data = root_group.load_data(&mut reader.file, 1)?;
            data.read_u32(0, &mut reader.file)?
        };

        let meta_data = {
            let data = root_group.load_data(&mut reader.file, 3)?;
            let mut buffer = vec![0u8; data.size as usize];
            data.read(0, &mut reader.file, &mut buffer)?;
            let text = String::from_utf8(buffer).map_err(ParsingError::FromUtf8Error)?;

            MetaData::deserialize(&text)
        };

        let (time_samplings, max_samples) = {
            let data = root_group.load_data(&mut reader.file, 4)?;
            time_sampling::read_time_samplings_and_max(&data, &mut reader.file)?
        };

        let indexed_meta_data = {
            let data = root_group.load_data(&mut reader.file, 5)?;
            metadata::read_indexed_meta_data(&data, &mut reader.file)?
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

    pub fn load_root_object(&self, reader: &mut FileReader) -> Result<ObjectReader> {
        let group = Rc::new(self.root_group.load_group(&mut reader.file, 2, false)?);
        ObjectReader::new(
            group,
            "",
            &mut reader.file,
            &self.indexed_meta_data,
            &self.time_samplings,
            Rc::new(self.root_header.clone()),
        )
    }
}
