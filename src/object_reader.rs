use crate::reader::{ArchiveReader, StringReader};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::rc::Rc;

use crate::chunks::*;
use crate::metadata::*;
use crate::property::*;
use crate::result::*;
use crate::time_sampling::TimeSampling;

#[derive(Debug, Clone)]
pub struct ObjectHeader {
    pub name: String,
    pub full_name: String,
    pub meta_data: MetaData,
}

#[derive(Debug)]
pub struct ObjectReader {
    pub header: ObjectHeader,
    pub group: GroupChunk,
    pub cp_reader: Option<CompoundPropertyReader>,
    pub children: Vec<ObjectHeader>,
    pub child_map: HashMap<String, usize>,
}
impl ObjectReader {
    pub fn new(
        group: GroupChunk,
        parent_name: &str,
        reader: &mut dyn ArchiveReader,
        indexed_meta_data: &[MetaData],
        time_samplings: &[Rc<TimeSampling>],
        header: ObjectHeader,
    ) -> Result<Self> {
        let child_count = group.child_count as usize;

        let mut cp_reader = None;

        let mut children = Vec::default();
        let mut child_map = HashMap::default();

        if child_count > 0 {
            if is_data(group.children[child_count - 1]) {
                children = read_object_headers(
                    &group,
                    child_count - 1,
                    parent_name,
                    reader,
                    indexed_meta_data,
                )?;

                for (i, child) in children.iter().enumerate() {
                    child_map.insert(child.name.clone(), i);
                }
                assert!(child_map.len() == children.len());
            }

            if is_group(group.children[0]) {
                let child_group = group.load_group(reader, 0, false)?;
                cp_reader = Some(CompoundPropertyReader::new(
                    child_group,
                    header.meta_data.clone(),
                    reader,
                    indexed_meta_data,
                    time_samplings,
                )?);
            }
        }

        Ok(Self {
            header,
            group,
            cp_reader,
            children,
            child_map,
        })
    }

    pub fn child_count(&self) -> usize {
        self.child_map.len()
    }

    pub fn load_child(
        &self,
        index: usize,
        reader: &mut dyn ArchiveReader,
        indexed_meta_data: &[MetaData],
        time_samplings: &[Rc<TimeSampling>],
    ) -> Result<ObjectReader> {
        let parent_group = &self.group;
        let child_group = parent_group.load_group(reader, index + 1, false)?;

        ObjectReader::new(
            child_group,
            &self.children[index].full_name,
            reader,
            indexed_meta_data,
            time_samplings,
            self.children[index].clone(),
        )
    }

    pub fn properties(&self) -> Option<&CompoundPropertyReader> {
        self.cp_reader.as_ref()
    }
}

fn read_object_headers(
    group: &GroupChunk,
    index: usize,
    parent_name: &str,
    reader: &mut dyn ArchiveReader,
    indexed_meta_data: &[MetaData],
) -> Result<Vec<ObjectHeader>> {
    let data = group.load_data(reader, index)?;

    if data.size <= 32 {
        return Ok(vec![]);
    }

    // skip the last 32 bytes which contains the hashes
    let mut buffer = vec![0u8; (data.size - 32) as usize];
    data.read(0, reader, &mut buffer)?;
    let buffer_size = buffer.len() as u64;
    let mut buffer = std::io::Cursor::new(buffer);

    let mut headers = vec![];

    loop {
        if buffer.position() == buffer_size {
            break;
        }

        let name_size = buffer.read_u32::<LittleEndian>()?;
        let name = buffer.read_string(name_size as usize)?;

        let meta_data_index = buffer.read_u8()? as usize;
        let meta_data = if meta_data_index == 0xff {
            let meta_data_size = buffer.read_u32::<LittleEndian>()?;
            let text = buffer.read_string(meta_data_size as usize)?;
            MetaData::deserialize(&text)
        } else if meta_data_index < indexed_meta_data.len() {
            indexed_meta_data[meta_data_index].clone()
        } else {
            return Err(ParsingError::InvalidAlembicFile.into());
        };

        let full_name = format!("{}/{}", parent_name, &name);
        headers.push(ObjectHeader {
            name,
            full_name,
            meta_data,
        });
    }

    // TODO(max): Verify hashes?

    Ok(headers)
}
