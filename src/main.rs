use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::rc::Rc;

pub(crate) mod result;
use result::{InternalError, OgawaError, ParsingError, Result};

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

struct ObjectHeader {
    name: String,
    full_name: String,
    meta_data: MetaData,
}

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

#[derive(Debug, Clone)]
struct Group {
    position: u64,
    child_count: u64, //needs to be a separate variable from the length of the children vec
    children: Vec<u64>,
}

impl Group {
    fn load(group_pos: u64, is_light: bool, reader: &mut BufReader<File>) -> Result<Group> {
        reader.seek(SeekFrom::Start(group_pos))?;

        let child_count = reader.read_u64::<LittleEndian>()?;

        if child_count > 123456 /*TODO(manon): replace with file size / 8?*/|| child_count == 0 {
            return Ok(Group {
                position: group_pos,
                child_count: 0,
                children: vec![],
            });
        }

        // defer loading larger number of children
        let children = if !is_light && child_count < 9 {
            (0..child_count)
                .map(|_| reader.read_u64::<LittleEndian>().map_err(|err| err.into()))
                .collect::<Result<Vec<_>>>()?
        } else {
            vec![]
        };

        Ok(Group {
            position: group_pos,
            child_count,
            children,
        })
    }
    fn is_child_group(&self, index: usize) -> bool {
        index < self.children.len() && is_group(self.children[index])
    }
    fn is_child_data(&self, index: usize) -> bool {
        index < self.children.len() && is_data(self.children[index])
    }
    fn is_empty_child_group(&self, index: usize) -> bool {
        index < self.children.len() && is_empty_group(self.children[index])
    }
    fn is_empty_child_data(&self, index: usize) -> bool {
        index < self.children.len() && is_empty_data(self.children[index])
    }
    fn is_light(&self) -> bool {
        self.child_count != 0 && self.children.is_empty()
    }

    fn load_group(
        &self,
        reader: &mut BufReader<File>,
        index: usize,
        is_light: bool,
    ) -> Result<Rc<Group>> {
        if self.is_light() {
            if index < (self.child_count as usize) {
                reader.seek(SeekFrom::Start(self.position + 8 * (index as u64) + 8))?;
                let child_pos = reader.read_u64::<LittleEndian>()?;

                if (child_pos & EMPTY_DATA) == 0 {
                    let group = Rc::new(Group::load(child_pos, is_light, reader)?);
                    Ok(group)
                } else {
                    Err(InternalError::DataReadAsGroup.into())
                }
            } else {
                Err(InternalError::OutOfBounds.into())
            }
        } else if self.is_child_group(index) {
            let child_pos = self.children[index];
            let group = Rc::new(Group::load(child_pos, is_light, reader)?);
            Ok(group)
        } else {
            Err(InternalError::DataReadAsGroup.into())
        }
    }

    fn load_data(&self, reader: &mut BufReader<File>, index: usize) -> Result<Rc<Data>> {
        if self.is_light() {
            if index < (self.child_count as usize) {
                reader.seek(SeekFrom::Start(self.position + 8 * (index as u64) + 8))?;
                let child_pos = reader.read_u64::<LittleEndian>()?;
                if (child_pos & EMPTY_DATA) != 0 {
                    Ok(Rc::new(Data::load(child_pos, reader)?))
                } else {
                    Err(InternalError::GroupReadAsData.into())
                }
            } else {
                Err(InternalError::OutOfBounds.into())
            }
        } else if self.is_child_data(index) {
            Ok(Rc::new(Data::load(self.children[index], reader)?))
        } else {
            Err(InternalError::GroupReadAsData.into())
        }
    }
}

#[derive(Debug)]
pub struct Data {
    position: u64,
    size: u64,
}

impl Data {
    fn load(position: u64, reader: &mut BufReader<File>) -> Result<Data> {
        let position = position & INVALID_GROUP;

        let size = if position != 0 {
            reader.seek(SeekFrom::Start(position))?;
            let size = reader.read_u64::<LittleEndian>()?;
            //TODO(max): return error if size is larger than file size
            size
        } else {
            0
        };

        Ok(Data { position, size })
    }

    fn read(&self, offset: u64, reader: &mut BufReader<File>, buffer: &mut [u8]) -> Result<()> {
        if self.size == 0
        /* || offset + size > file_size*/
        {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        reader.seek(SeekFrom::Start(self.position + offset + 8))?;
        reader.read(buffer)?;

        Ok(())
    }
    fn read_u32(&self, offset: u64, reader: &mut BufReader<File>) -> Result<u32> {
        if self.size != 4 {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        reader.seek(SeekFrom::Start(self.position + offset))?;
        let value = reader.read_u32::<LittleEndian>()?;
        Ok(value)
    }
}

#[derive(Debug)]
enum PropertyType {}
#[derive(Debug)]
enum DataType {}

#[derive(Debug)]
struct PropertyHeader {
    name: String,
    property_type: PropertyType,
    meta_data: MetaData,
    data_type: DataType,
    time_sampling: Rc<TimeSampling>,

    //friends?
    is_scalar_like: bool,
    is_homogenous: bool,
    next_sample_index: u32,
    first_changed_index: u32,
    last_changed_index: u32,
    time_sampling_index: u32,
}
#[derive(Debug)]
struct CprData {
    group: Rc<Group>,
    property_headers: Vec<PropertyHeader>,
    sub_properties: HashMap<String, usize>,
}
impl CprData {
    fn new(
        group: Rc<Group>,
        reader: &mut BufReader<File>,
        indexed_meta_data: &Vec<MetaData>,
    ) -> Result<Self> {
        let child_count = group.child_count as usize;
        let mut property_headers = vec![];
        let mut sub_properties = HashMap::default();

        dbg!(&group);
        println!("child_count: {}", child_count);

        if child_count > 0 && is_data(group.children[child_count - 1]) {
            property_headers =
                read_property_headers(&group, child_count - 1, reader, indexed_meta_data)?;
            for (i, header) in property_headers.iter().enumerate() {
                sub_properties.insert(header.name.clone(), i);
            }
        }

        Ok(Self {
            group,
            property_headers,
            sub_properties,
        })
    }
}

#[derive(Debug)]
struct OrData {
    group: Rc<Group>,
    data: Option<CprData>,
    children: Vec<ObjectHeader>,
    child_map: HashMap<String, usize>,
}
impl OrData {
    fn new(
        group: Rc<Group>,
        parent_name: &str,
        reader: &mut BufReader<File>,
        indexed_meta_data: &Vec<MetaData>,
    ) -> Result<Self> {
        let child_count = group.child_count as usize;

        let mut data = None;

        let mut children = Vec::default();
        let mut child_map = HashMap::default();

        if child_count > 0 {
            if is_data(group.children[child_count - 1]) {
                children = read_object_headers(
                    group.as_ref(),
                    child_count - 1,
                    parent_name,
                    indexed_meta_data,
                    reader,
                )?;

                for (i, child) in children.iter().enumerate() {
                    child_map.insert(child.name.clone(), i);
                }
            }

            if is_group(group.children[0]) {
                let child_group = group.load_group(reader, 0, false)?;
                data = Some(CprData::new(child_group, reader, indexed_meta_data)?);
            }
        }

        Ok(Self {
            group,
            data,
            children,
            child_map,
        })
    }
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

fn read_time_samples_and_max(
    data: &Data,
    reader: &mut BufReader<File>,
) -> Result<(Vec<TimeSampling>, Vec<i64>)> {
    let mut buffer = vec![0u8; data.size as usize];
    println!("data.size: {}", data.size);
    data.read(0, reader, &mut buffer)?;
    dbg!(&buffer);
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

        println!("max_sample: {}", max_sample);
        println!("time_per_cycle: {:+e}", time_per_cycle);
        println!("num_samples_per_cycle: {}", num_samples_per_cycle);

        let mut samples = vec![0.0f64; num_samples_per_cycle as usize];
        buffer
            .read_f64_into::<LittleEndian>(&mut samples)
            .map_err(|_| ParsingError::InvalidAlembicFile)?;

        dbg!(&samples);

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

        out_time_samples.push(TimeSampling {
            sampling_type,
            samples,
        });
        println!();
    }

    Ok((out_time_samples, out_max_samples))
}

fn read_object_headers(
    group: &Group,
    index: usize,
    parent_name: &str,
    indexed_meta_data: &Vec<MetaData>,
    reader: &mut BufReader<File>,
) -> Result<Vec<ObjectHeader>> {
    let data = group.load_data(reader, index)?;

    println!("position: 0x{:x}", data.position);

    if data.size <= 32 {
        return Err(ParsingError::InvalidAlembicFile.into());
    }

    let mut buffer = vec![0u8; data.size as usize];
    data.read(0, reader, &mut buffer)?;
    let mut buffer = std::io::Cursor::new(buffer);

    let mut headers = vec![];

    loop {
        if buffer.position() == data.size - 32 {
            break;
        }

        dbg!("mes");
        let name_size = buffer.read_u32::<LittleEndian>()?;
        dbg!(name_size);
        let name = buffer.read_string(name_size as usize)?;

        dbg!(&name);

        let meta_data_index = buffer.read_u8()? as usize;
        let meta_data = if meta_data_index == 0xff {
            dbg!(meta_data_index);
            let meta_data_size = buffer.read_u32::<LittleEndian>()?;
            let text = buffer.read_string(meta_data_size as usize)?;
            MetaData::deserialize(&text)
        } else if meta_data_index < indexed_meta_data.len() {
            indexed_meta_data[meta_data_index].clone()
        } else {
            return Err(ParsingError::InvalidAlembicFile.into());
        };

        let full_name = format!("{}/{}", &parent_name, &name);
        headers.push(ObjectHeader {
            name,
            full_name,
            meta_data,
        });
    }

    dbg!("hallo");
    Ok(headers)
}

fn read_property_headers(
    group: &Group,
    index: usize,
    reader: &mut BufReader<File>,
    indexed_meta_data: &Vec<MetaData>,
) -> Result<Vec<PropertyHeader>> {
    Ok(vec![])
}

fn print_debug_info(root_group: &Group, reader: &mut BufReader<File>) -> Result<()> {
    let mut total_data_size = 0;
    let mut data_count = 0;
    let mut group_count = 0;
    let mut stack = vec![(0, 0, Rc::new(root_group.clone()))];

    loop {
        if stack.len() == 0 {
            break;
        }

        let (indent, index, current) = stack.pop().unwrap();
        group_count += 1;

        for _ in 0..indent {
            print!("    ");
        }
        println!(
            "({}) group: 0x{:016x} ({} children)",
            index, current.position, current.child_count
        );

        for (i, child) in current.children.iter().enumerate() {
            if is_group(*child) {
                let group = current.load_group(reader, i, false)?;
                stack.push((indent + 1, i, group))
            } else if is_data(*child) {
                let data = current.load_data(reader, i)?;

                for _ in 0..=indent {
                    print!("    ");
                }
                println!(
                    "({}) data: 0x{:016x} ({} bytes)",
                    i, data.position, data.size
                );

                total_data_size += data.size;
                data_count += 1;
            }
        }
    }

    dbg!(total_data_size);
    dbg!(data_count);
    dbg!(group_count);
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
    file.read(&mut magic)?;
    dbg!(&magic);
    if &magic != &[0x4f, 0x67, 0x61, 0x77, 0x61] {
        return Err(anyhow::anyhow!("Magic value of ogawa file does not match.").into());
    }

    let frozen = file.read_u8()? == 0xff;
    dbg!(frozen);

    let file_version = file.read_u16::<LittleEndian>()?;
    dbg!(file_version);

    let group_pos = file.read_u64::<LittleEndian>()?;
    dbg!(group_pos);

    let root_group = Group::load(group_pos, false, &mut file)?;

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

    let file_version = {
        let data = root_group.load_data(&mut file, 1)?;
        data.read_u32(0, &mut file)?
    };
    dbg!(file_version);

    println!("kaas");
    let (time_samples, max_samples) = {
        println!("time sampling location: 0x{:x}", root_group.children[4]);
        let data = root_group.load_data(&mut file, 4)?;
        read_time_samples_and_max(&data, &mut file)?
    };

    println!("worst");
    let indexed_meta_data = {
        println!("meta data location: 0x{:x}", root_group.children[5]);
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

    dbg!(&time_samples);
    dbg!(&max_samples);

    dbg!(&header);
    print_debug_info(&root_group, &mut file)?;

    println!("a");
    let group = root_group.load_group(&mut file, 2, false)?;
    println!("b");
    let or_data = OrData::new(group, "", &mut file, &indexed_meta_data)?;
    println!("c");
    dbg!(&or_data);

    dbg!(header.meta_data.serialize());

    /*
    for child in root_group.children.iter() {
        let description = match *child {
            INVALID_DATA => "INVALID_DATA",
            INVALID_GROUP => "INVALID_GROUP",
            EMPTY_DATA => "EMPTY_DATA",
            EMPTY_GROUP => "EMPTY_GROUP",
            _ if *child & EMPTY_DATA == EMPTY_DATA => "DATA",
            _ => "GROUP",
        };

        println!("child: 0x{:016x} ({})", child, description);
    }

    for (i, child) in root_group.children.iter().enumerate() {
        if is_data(*child) {
            let data = root_group.load_data(&mut file, i)?;
            if let Some(data) = data {
                dbg!(&data);
                let mut buffer = vec![0u8; data.size as usize];
                data.read(data.size, data.position, &mut file, &mut buffer)?;
                //dbg!(buffer);
            }
        }
    }*/

    //dbg!(&root_group);
    /*
    //Read first header line
    let mut header = vec![0u8; 16];
    file.read(&mut header)?;

    if &header[0..5] != &[0x4f, 0x67, 0x61, 0x77, 0x61] {
        return Err(anyhow::anyhow!("Magic value of ogawa file does not match.").into());
    }

    let frozen = header[5] == 0xff;
    let file_version = ((header[6] as u16) << 8) | (header[7] as u16);
    let group_pos = header[8..].read;
    dbg!(frozen);
    dbg!(file_version);
    */

    Ok(())
}
