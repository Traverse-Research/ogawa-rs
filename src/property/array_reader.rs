use super::PropertyHeader;
use crate::*;

#[derive(Debug)]
pub(crate) struct ArrayPropertyReader {
    pub(crate) group: Rc<GroupChunk>,
    pub(crate) header: PropertyHeader,
}
impl ArrayPropertyReader {
    pub(crate) fn new(group: Rc<GroupChunk>, header: PropertyHeader) -> Self {
        Self { group, header }
    }

    pub(crate) fn name(&self) -> &str {
        &self.header.name
    }
}
