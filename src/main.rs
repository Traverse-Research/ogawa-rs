use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::rc::Rc;

mod object_reader;
use object_reader::*;
mod property;
use property::*;
mod chunks;
use chunks::*;

pub(crate) mod result;
use result::{InternalError, OgawaError, ParsingError, Result, UserError};

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

fn is_group(value: u64) -> bool {
    (value & EMPTY_DATA) == 0
}
fn is_data(value: u64) -> bool {
    !is_group(value)
}
fn is_empty_group(value: u64) -> bool {
    value == EMPTY_GROUP
}
fn is_empty_data(value: u64) -> bool {
    value == EMPTY_DATA
}
fn address_from_child(child: u64) -> u64 {
    child & INVALID_GROUP
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum PodType {
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
enum PodArray {
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
struct DataType {
    pod_type: PodType,
    extent: u8,
}

const ACYCLIC_NUM_SAMPLES: u32 = u32::MAX;
const ACYCLIC_TIME_PER_CYCLE: f64 = f64::MAX / 32.0;
#[derive(Debug)]
struct TimeSamplingType {
    num_samples_per_cycle: u32,
    time_per_cycle: f64,
}
#[derive(Debug)]
struct TimeSampling {
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

fn print_debug_info(root_group: &GroupChunk, reader: &mut BufReader<File>) -> Result<()> {
    let mut total_data_size = 0;
    let mut data_count = 0;
    let mut group_count = 0;
    let mut stack = vec![(0, 0, Chunk::Group(root_group.clone()))];

    loop {
        if stack.is_empty() {
            break;
        }

        let (indent, index, current) = stack.pop().unwrap();
        group_count += 1;

        for _ in 0..indent {
            print!("|   ");
        }

        match &current {
            Chunk::Group(group) => {
                println!(
                    "({}) group: 0x{:016x} ({} children)",
                    index, group.position, group.child_count
                );
            }
            Chunk::Data(data) => {
                println!(
                    "({}) data: 0x{:016x} ({} bytes)",
                    index, data.position, data.size
                );
            }
        }

        if let Chunk::Group(current_group) = &current {
            for (i, &child) in current_group.children.iter().enumerate().rev() {
                if is_group(child) {
                    let group = current_group.load_group(reader, i, false)?;
                    stack.push((indent + 1, i, Chunk::Group(group)));
                } else {
                    let data = current_group.load_data(reader, i)?;

                    total_data_size += data.size;
                    data_count += 1;

                    stack.push((indent + 1, i, Chunk::Data(data)));
                }
            }
        }
    }

    println!("total_data_size: {}", total_data_size);
    println!("data_count: {}", data_count);
    println!("group_count: {}", group_count);
    Ok(())
}

fn main() -> Result<(), OgawaError> {
    println!("Hello, world!");

    let file_name = "test_assets/Eyelashes01.abc";
    //let file_name = "test_assets/Eyebrows01.abc";
    //let file_name = "test_assets/Mainhair01.abc";
    let file = File::open(file_name)?;
    let mut file = BufReader::new(file);

    let file_size = file.seek(SeekFrom::End(0))?;
    let _ = file.seek(SeekFrom::Start(0));
    dbg!(file_size);

    let mut magic = vec![0; 5];
    file.read_exact(&mut magic)?;
    if magic != [0x4f, 0x67, 0x61, 0x77, 0x61] {
        return Err(anyhow::anyhow!("Magic value of ogawa file does not match.").into());
    }

    let _frozen = file.read_u8()? == 0xff;
    let alembic_file_version = file.read_u16::<LittleEndian>()?;
    if alembic_file_version >= 9999 {
        return Err(ParsingError::UnsupportedAlembicFile.into());
    }
    let group_pos = file.read_u64::<LittleEndian>()?;

    let root_group = GroupChunk::load(group_pos, false, &mut file)?;

    if root_group.child_count <= 5
        || !is_data(root_group.children[0] /*  version */)
        || !is_data(root_group.children[1] /*  file version */)
        || !is_group(root_group.children[2] /* root? */)
        || !is_data(root_group.children[3] /*  ??? */)
        || !is_data(root_group.children[4] /*  time sampling */)
        || !is_data(root_group.children[5] /*  metadata */)
    {
        return Err(anyhow::anyhow!("Invalid Alembic file.").into());
    }

    let version = {
        let data = root_group.load_data(&mut file, 0)?;
        data.read_u32(0, &mut file)?
    };
    dbg!(version);

    let ogawa_file_version = {
        let data = root_group.load_data(&mut file, 1)?;
        data.read_u32(0, &mut file)?
    };
    dbg!(ogawa_file_version);

    let (time_samplings, _max_samples) = {
        let data = root_group.load_data(&mut file, 4)?;
        read_time_samplings_and_max(&data, &mut file)?
    };

    let indexed_meta_data = {
        let data = root_group.load_data(&mut file, 5)?;
        metadata::read_indexed_meta_data(&data, &mut file)?
    };

    let meta_data = {
        let data = root_group.load_data(&mut file, 3)?;
        let mut buffer = vec![0u8; data.size as usize];
        data.read(0, &mut file, &mut buffer)?;
        let text = String::from_utf8(buffer)?;

        MetaData::deserialize(&text)
    };

    let header = ObjectHeader {
        name: "ABC".to_owned(),
        full_name: "/".to_owned(),
        meta_data,
    };

    print_debug_info(&root_group, &mut file)?;

    let group = Rc::new(root_group.load_group(&mut file, 2, false)?);
    let object_reader = ObjectReader::new(
        group,
        "",
        &mut file,
        &indexed_meta_data,
        &time_samplings,
        Rc::new(header),
    )?;

    let mut stack = vec![(0, Rc::new(object_reader))];

    loop {
        if stack.is_empty() {
            break;
        }

        let (indent, current) = stack.pop().unwrap();
        let header = current.header.clone();
        for _ in 0..indent {
            print!("|   ");
        }
        println!("name: {}", &header.full_name);

        let child_count = current.child_map.len();
        for i in 0..child_count {
            let child = current.load_child(i, &mut file, &indexed_meta_data, &time_samplings)?;
            stack.push((indent + 1, child));
        }

        let properties = current.properties().unwrap();
        let mut prop_stack = vec![];
        for i in (0..properties.sub_property_count()).rev() {
            let prop =
                properties.load_sub_property(i, &mut file, &indexed_meta_data, &time_samplings)?;
            prop_stack.push((1, Rc::new(prop)));
        }

        loop {
            if prop_stack.is_empty() {
                break;
            }

            let (prop_indent, properties) = prop_stack.pop().unwrap();

            if let PropertyReader::Compound(properties) = properties.as_ref() {
                for i in (0..properties.sub_property_count()).rev() {
                    let prop = properties.load_sub_property(
                        i,
                        &mut file,
                        &indexed_meta_data,
                        &time_samplings,
                    )?;

                    prop_stack.push(((prop_indent + 1), Rc::new(prop)));
                }
            }

            let prop_name = properties.name().to_owned();
            let typename = match properties.as_ref() {
                PropertyReader::Array(_) => "array",
                PropertyReader::Compound(_) => "compound",
                PropertyReader::Scalar(_) => "scalar",
            };
            for _ in 0..(indent + prop_indent) {
                print!("|   ");
            }
            println!("prop({}): {}", typename, prop_name);

            match properties.as_ref() {
                PropertyReader::Scalar(pr) => {
                    for i in 0..pr.sample_count() {
                        for _ in 0..(indent + prop_indent + 1) {
                            print!("|   ");
                        }
                        let size = pr.sample_size(i, &mut file)?;
                        print!("scalar data {:?} ({} bytes)", &pr.header.data_type, size);
                        let sample = pr.load_sample(i, &mut file)?;
                        print!("{:?}", &sample);
                        println!();
                    }
                }
                PropertyReader::Array(pr) => {
                    for i in 0..pr.sample_count() {
                        for _ in 0..(indent + prop_indent + 1) {
                            print!("|   ");
                        }
                        let size = pr.sample_size(i, &mut file)?;
                        print!("array data {:?} ({} bytes)", &pr.header.data_type, size);
                        let _sample = pr.load_sample(i, &mut file)?;
                        //print!("{:?}", &sample.len());
                        println!();
                    }
                }
                PropertyReader::Compound(_) => {}
            }
        }
    }

    Ok(())
}
