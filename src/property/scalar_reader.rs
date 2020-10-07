use super::PropertyHeader;
use crate::*;

#[derive(Debug)]
pub(crate) struct ScalarPropertyReader {
    pub(crate) group: Rc<GroupChunk>,
    pub(crate) header: PropertyHeader,
}

impl ScalarPropertyReader {
    pub(crate) fn new(group: Rc<GroupChunk>, header: PropertyHeader) -> Self {
        Self { group, header }
    }
    pub(crate) fn name(&self) -> &str {
        &self.header.name
    }

    pub(crate) fn sample_count(&self) -> u32 {
        self.header.next_sample_index
    }
    pub(crate) fn load_sample(&self, index: u32) -> Vec<u8> {
        todo!();
    }
}
