use super::PropertyHeader;
use crate::*;

#[derive(Debug)]
pub struct ArrayPropertyReader {
    pub group: Rc<GroupChunk>,
    pub header: PropertyHeader,
}

impl ArrayPropertyReader {
    pub fn new(group: Rc<GroupChunk>, header: PropertyHeader) -> Self {
        Self { group, header }
    }
    pub fn name(&self) -> &str {
        &self.header.name
    }

    pub fn sample_count(&self) -> u32 {
        self.header.next_sample_index
    }
    pub fn load_sample(&self, index: u32, reader: &mut BufReader<File>) -> Result<PodArray> {
        if index >= self.header.next_sample_index {
            return Err(UserError::OutOfBounds.into());
        }

        let index = self.header.map_index(index);
        let data = self.group.load_data(reader, index)?;
        data.read_pod_array(&self.header.data_type, reader)
    }
    pub fn sample_size(&self, index: u32, reader: &mut BufReader<File>) -> Result<usize> {
        if index >= self.header.next_sample_index {
            return Err(UserError::OutOfBounds.into());
        }

        let index = self.header.map_index(index);
        let data = self.group.load_data(reader, index)?;
        //data.read_pod(&self.header.data_type, self.header.data_type.pod_type)
        Ok(data.size as usize)
    }
}
