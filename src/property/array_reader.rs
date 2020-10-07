use super::PropertyHeader;
use crate::*;

#[derive(Debug)]
pub(crate) struct ArrayPropertyReader {
    pub(crate) group: Rc<Group>,
    pub(crate) header: PropertyHeader,
}
impl ArrayPropertyReader {
    pub(crate) fn new(group: Rc<Group>, header: PropertyHeader) -> Self {
        Self { group, header }
    }
}
