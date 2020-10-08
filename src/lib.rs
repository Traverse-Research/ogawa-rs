use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::rc::Rc;

mod object_reader;
pub use object_reader::{ObjectHeader, ObjectReader};
mod property;
//use property::*;
pub use property::*;
mod chunks;
pub use chunks::*;

mod result;
pub use result::{InternalError, OgawaError, ParsingError, Result, UserError};

mod metadata;
use metadata::MetaData;
trait StringReader {
    fn read_string(&mut self, size: usize) -> Result<String>;
}
impl StringReader for std::io::Cursor<Vec<u8>> {
    fn read_string(&mut self, size: usize) -> Result<String> {
        let mut buffer = vec![0u8; size];
        self.read(&mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

const INVALID_GROUP: u64 = 0x7fffffffffffffff;
const EMPTY_GROUP: u64 = 0x0000000000000000;
const INVALID_DATA: u64 = 0xffffffffffffffff;
const EMPTY_DATA: u64 = 0x8000000000000000;

#[derive(Debug)]
struct ArchiveReader {
    filename: String,
    stream_count: usize,

    archive_version: i32,
    header: ObjectHeader,
}

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

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PodType {
    Boolean = 0,
    U8,
    I8,

    U16,
    I16,

    U32,
    I32,

    U64,
    I64,

    F16,
    F32,
    F64,

    String,
    WString,

    Unknown = 127,
}

#[derive(Debug)]
pub enum PodArray {
    Boolean(Vec<bool>),
    U8(Vec<u8>),
    I8(Vec<i8>),

    U16(Vec<u16>),
    I16(Vec<i16>),

    U32(Vec<u32>),
    I32(Vec<i32>),

    U64(Vec<u64>),
    I64(Vec<i64>),

    F16(Vec<half::f16>),
    F32(Vec<f32>),
    F64(Vec<f64>),

    String(Vec<String>),
    WString(Vec<String>),
}

use std::convert::{TryFrom, TryInto};
impl TryFrom<u32> for PodType {
    type Error = OgawaError;

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == PodType::Boolean as u32 => Ok(PodType::Boolean),
            x if x == PodType::U8 as u32 => Ok(PodType::U8),
            x if x == PodType::I8 as u32 => Ok(PodType::I8),
            x if x == PodType::U16 as u32 => Ok(PodType::U16),
            x if x == PodType::I16 as u32 => Ok(PodType::I16),
            x if x == PodType::U32 as u32 => Ok(PodType::U32),
            x if x == PodType::I32 as u32 => Ok(PodType::I32),
            x if x == PodType::U64 as u32 => Ok(PodType::U64),
            x if x == PodType::I64 as u32 => Ok(PodType::I64),

            x if x == PodType::F16 as u32 => Ok(PodType::F16),
            x if x == PodType::F32 as u32 => Ok(PodType::F32),
            x if x == PodType::F64 as u32 => Ok(PodType::F64),

            x if x == PodType::String as u32 => Ok(PodType::String),
            x if x == PodType::WString as u32 => Ok(PodType::WString),

            x if x == PodType::Unknown as u32 => Ok(PodType::Unknown),

            _ => Err(ParsingError::UnsupportedAlembicFile.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataType {
    pod_type: PodType,
    extent: u8,
}

const ACYCLIC_NUM_SAMPLES: u32 = u32::MAX;
const ACYCLIC_TIME_PER_CYCLE: f64 = f64::MAX / 32.0;
#[derive(Debug)]
pub struct TimeSamplingType {
    num_samples_per_cycle: u32,
    time_per_cycle: f64,
}
#[derive(Debug)]
pub struct TimeSampling {
    sampling_type: TimeSamplingType,
    samples: Vec<f64>,
}

fn read_time_samplings_and_max(
    data: &DataChunk,
    reader: &mut BufReader<File>,
) -> Result<(Vec<Rc<TimeSampling>>, Vec<i64>)> {
    let mut buffer = vec![0u8; data.size as usize];
    data.read(0, reader, &mut buffer)?;
    let mut buffer = std::io::Cursor::new(buffer);

    let mut out_max_samples = vec![];
    let mut out_time_samples = vec![];

    loop {
        if buffer.position() == data.size {
            break;
        }

        let max_sample = buffer.read_u32::<LittleEndian>()?;
        out_max_samples.push(max_sample as i64);
        let time_per_cycle = buffer.read_f64::<LittleEndian>()?;
        let num_samples_per_cycle = buffer.read_u32::<LittleEndian>()?;

        let mut samples = vec![0.0f64; num_samples_per_cycle as usize];
        buffer
            .read_f64_into::<LittleEndian>(&mut samples)
            .map_err(|_| ParsingError::InvalidAlembicFile)?;

        let sampling_type = if time_per_cycle == f64::MAX / 32.0 {
            TimeSamplingType {
                num_samples_per_cycle: ACYCLIC_NUM_SAMPLES,
                time_per_cycle: ACYCLIC_TIME_PER_CYCLE,
            }
        } else {
            TimeSamplingType {
                num_samples_per_cycle,
                time_per_cycle,
            }
        };

        out_time_samples.push(Rc::new(TimeSampling {
            sampling_type,
            samples,
        }));
    }

    Ok((out_time_samples, out_max_samples))
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
        dbg!(version);

        let ogawa_file_version = {
            let data = root_group.load_data(&mut reader.file, 1)?;
            data.read_u32(0, &mut reader.file)?
        };
        dbg!(ogawa_file_version);

        let (time_samplings, max_samples) = {
            let data = root_group.load_data(&mut reader.file, 4)?;
            read_time_samplings_and_max(&data, &mut reader.file)?
        };

        let indexed_meta_data = {
            let data = root_group.load_data(&mut reader.file, 5)?;
            metadata::read_indexed_meta_data(&data, &mut reader.file)?
        };

        let meta_data = {
            let data = root_group.load_data(&mut reader.file, 3)?;
            let mut buffer = vec![0u8; data.size as usize];
            data.read(0, &mut reader.file, &mut buffer)?;
            let text = String::from_utf8(buffer)?;

            MetaData::deserialize(&text)
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
}
