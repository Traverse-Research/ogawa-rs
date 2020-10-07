use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::rc::Rc;

use std::fs::File;
use std::io::BufReader;

use super::{PropertyReader, PropertyType};
use crate::*;

#[derive(Debug)]
pub(crate) struct CompoundPropertyReader {
    pub(crate) group: Rc<GroupChunk>,
    pub(crate) property_headers: Vec<PropertyHeader>,
    pub(crate) sub_properties: HashMap<String, usize>,
    pub(crate) header: PropertyHeader,
}

impl CompoundPropertyReader {
    pub(crate) fn new(
        group: Rc<GroupChunk>,
        meta_data: MetaData,
        reader: &mut BufReader<File>,
        indexed_meta_data: &Vec<MetaData>,
        time_samplings: &Vec<Rc<TimeSampling>>,
    ) -> Result<Self> {
        let child_count = group.child_count as usize;
        let mut property_headers = vec![];
        let mut sub_properties = HashMap::default();

        if child_count > 0 && is_data(group.children[child_count - 1]) {
            property_headers = read_property_headers(
                &group,
                child_count - 1,
                reader,
                indexed_meta_data,
                time_samplings,
            )?;

            for (i, header) in property_headers.iter().enumerate() {
                sub_properties.insert(header.name.clone(), i);
            }
        }

        let header = PropertyHeader {
            name: "".to_owned(),
            property_type: PropertyType::Compound,
            meta_data,
            data_type: DataType {
                pod_type: PodType::Unknown,
                extent: 0,
            },
            time_sampling: None,

            is_scalar_like: true,
            is_homogenous: true,
            next_sample_index: 0,
            first_changed_index: 0,
            last_changed_index: 0,
            time_sampling_index: 0,
        };

        Ok(Self {
            group,
            property_headers,
            sub_properties,
            header,
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.header.name
    }

    pub(crate) fn find_sub_property_index(&self, name: &str) -> Option<usize> {
        self.sub_properties.get(name).copied()
    }
    pub(crate) fn sub_property_count(&self) -> usize {
        self.property_headers.len()
    }
    pub(crate) fn load_sub_property(
        &self,
        index: usize,
        reader: &mut BufReader<File>,
        indexed_meta_data: &Vec<MetaData>,
        time_samplings: &Vec<Rc<TimeSampling>>,
    ) -> Result<PropertyReader> {
        let header = self
            .property_headers
            .get(index)
            .ok_or(UserError::OutOfBounds)?;

        let group = Rc::new(self.group.load_group(reader, index, false)?);
        Ok(match header.property_type {
            PropertyType::Array => {
                PropertyReader::Array(ArrayPropertyReader::new(group, header.clone()))
            }
            PropertyType::Compound => PropertyReader::Compound(CompoundPropertyReader::new(
                group,
                header.meta_data.clone(),
                reader,
                indexed_meta_data,
                time_samplings,
            )?),
            PropertyType::Scalar => {
                PropertyReader::Scalar(ScalarPropertyReader::new(group, header.clone()))
            }
        })
    }
}

fn read_property_headers(
    group: &GroupChunk,
    index: usize,
    reader: &mut BufReader<File>,
    indexed_meta_data: &Vec<MetaData>,
    time_samplings: &Vec<Rc<TimeSampling>>,
) -> Result<Vec<PropertyHeader>> {
    let data = group.load_data(reader, index)?;
    if data.size == 0 {
        return Ok(vec![]);
    }

    let mut buffer = vec![0u8; data.size as usize];
    data.read(0, reader, &mut buffer)?;
    let mut buffer = std::io::Cursor::new(buffer);

    let read_u32_with_hint =
        |buffer: &mut std::io::Cursor<Vec<u8>>, size_hint: u32| -> Result<u32> {
            match size_hint {
                0 => Ok(buffer.read_u8()? as u32),
                1 => Ok(buffer.read_u16::<LittleEndian>()? as u32),
                2 => Ok(buffer.read_u32::<LittleEndian>()?),
                _ => Err(InternalError::Unreachable.into()),
            }
        };

    let mut output_headers = vec![];
    loop {
        if buffer.position() == data.size {
            break;
        }

        let info = buffer.read_u32::<LittleEndian>()?;
        let property_type = info & 0x3;
        let is_scalar_like = (property_type & 0x1) != 0;
        let property_type = match property_type {
            0 => PropertyType::Compound,
            1 => PropertyType::Scalar,
            _ => PropertyType::Array,
        };

        let size_hint = (info >> 2) & 0x3;

        let mut time_sampling = None;
        let mut is_homogenous = false;
        let mut next_sample_index = 0;
        let mut first_changed_index = 0;
        let mut last_changed_index = 0;
        let mut time_sampling_index = 0;
        let mut data_type = DataType {
            pod_type: PodType::Unknown,
            extent: 0,
        };

        if property_type != PropertyType::Compound {
            let pod_type = (info >> 4) & 0xf;
            if pod_type >= PodType::NumPodTypes as u32 {
                return Err(ParsingError::InvalidAlembicFile.into());
            }
            let pod_type: PodType = pod_type.try_into()?;
            let extent = ((info >> 12) & 0xff) as u8;
            data_type = DataType { pod_type, extent };

            is_homogenous = (info & 0x400) != 0;
            next_sample_index = read_u32_with_hint(&mut buffer, size_hint)?;

            if (info & 0x0200) != 0 {
                first_changed_index = read_u32_with_hint(&mut buffer, size_hint)?;
                last_changed_index = read_u32_with_hint(&mut buffer, size_hint)?;
            } else if (info & 0x0800) != 0 {
                first_changed_index = 0;
                last_changed_index = 0;
            } else {
                first_changed_index = 0;
                last_changed_index = next_sample_index - 1;
            };

            time_sampling_index = if (info & 0x0100) != 0 {
                read_u32_with_hint(&mut buffer, size_hint)?
            } else {
                0
            };

            if (time_sampling_index as usize) >= time_samplings.len() {
                return Err(ParsingError::InvalidAlembicFile.into());
            }

            time_sampling = Some(Rc::clone(&time_samplings[time_sampling_index as usize]));
        }

        let name_size = read_u32_with_hint(&mut buffer, size_hint)?;
        let name = buffer.read_string(name_size as usize)?;

        let meta_data_index = ((info >> 20) & 0xff) as usize;
        let meta_data = if meta_data_index == 0xff {
            let meta_data_size = read_u32_with_hint(&mut buffer, size_hint)?;
            if buffer.position() == data.size {
                MetaData::default()
            } else {
                let text = buffer.read_string(meta_data_size as usize)?;
                MetaData::deserialize(&text)
            }
        } else if meta_data_index < indexed_meta_data.len() {
            indexed_meta_data[meta_data_index].clone()
        } else {
            return Err(ParsingError::InvalidAlembicFile.into());
        };

        output_headers.push(PropertyHeader {
            name,
            property_type,
            meta_data,
            data_type,
            time_sampling,
            is_scalar_like,
            is_homogenous,
            next_sample_index,
            first_changed_index,
            last_changed_index,
            time_sampling_index,
        });
    }

    Ok(output_headers)
}
