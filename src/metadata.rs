use crate::reader::ArchiveReader;
use crate::result::*;
use byteorder::ReadBytesExt;
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, Default)]
pub struct MetaData {
    pub tokens: BTreeMap<String, String>,
}
impl MetaData {
    pub fn deserialize(text: &str) -> MetaData {
        let mut tokens = BTreeMap::new();

        let mut last_pair = 0;
        loop {
            let cur_pair = text[last_pair..].find(';').map(|x| x + last_pair);
            let cur_assign = text[last_pair..].find('=').map(|x| x + last_pair);

            if let Some(cur_assign) = cur_assign {
                let cur_pair = cur_pair.unwrap_or(text.len());
                let key = text[last_pair..cur_assign].to_owned();
                let value = if (cur_assign + 1) > cur_pair {
                    ""
                } else {
                    &text[(cur_assign + 1)..cur_pair]
                }
                .to_owned();

                tokens.insert(key, value);
            }

            if let Some(cur_pair) = cur_pair {
                last_pair = cur_pair + 1;
            } else {
                break;
            }
        }

        MetaData { tokens }
    }

    pub fn serialize(&self) -> String {
        self.tokens
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .fold("".to_owned(), |a, b| {
                if a.is_empty() {
                    b
                } else {
                    format!("{};{}", a, b)
                }
            })
    }
}

pub(crate) fn read_indexed_meta_data(
    data: &crate::DataChunk,
    reader: &mut dyn ArchiveReader,
) -> Result<Vec<MetaData>> {
    let mut output = vec![MetaData::default()];

    let mut buffer = vec![0; data.size as usize];
    data.read(0, reader, &mut buffer)?;
    let mut buffer = std::io::Cursor::new(buffer);

    loop {
        if buffer.position() == data.size {
            break;
        }

        let meta_data_size = buffer.read_u8()?;

        let meta_data = if buffer.position() + meta_data_size as u64 == data.size {
            buffer.seek(SeekFrom::Current(meta_data_size as i64))?;
            MetaData {
                tokens: BTreeMap::new(),
            }
        } else {
            let mut string_buffer = vec![0u8; meta_data_size as usize];
            buffer.read_exact(&mut string_buffer)?;
            let text = String::from_utf8(string_buffer).map_err(ParsingError::FromUtf8Error)?;
            MetaData::deserialize(&text)
        };
        output.push(meta_data);
    }

    Ok(output)
}
