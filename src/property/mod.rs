pub(crate) mod array_reader;
pub(crate) mod compound_reader;
pub(crate) mod scalar_reader;

pub(crate) use array_reader::*;
pub(crate) use compound_reader::*;
pub(crate) use scalar_reader::*;

use std::rc::Rc;

use crate::metadata::MetaData;
use crate::DataType;
use crate::TimeSampling;

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum PropertyType {
    Array,
    Compound,
    Scalar,
}

pub(crate) enum PropertyReader {
    Array(ArrayPropertyReader),
    Compound(CompoundPropertyReader),
    Scalar(ScalarPropertyReader),
}
impl PropertyReader {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Array(r) => r.name(),
            Self::Compound(r) => r.name(),
            Self::Scalar(r) => r.name(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PropertyHeader {
    pub(crate) name: String,
    pub(crate) property_type: PropertyType,
    pub(crate) meta_data: MetaData,
    pub(crate) data_type: DataType,
    pub(crate) time_sampling: Option<Rc<TimeSampling>>,

    //friends?
    pub(crate) is_scalar_like: bool,
    pub(crate) is_homogenous: bool,
    pub(crate) next_sample_index: u32,
    pub(crate) first_changed_index: u32,
    pub(crate) last_changed_index: u32,
    pub(crate) time_sampling_index: u32,
}

impl PropertyHeader {
    fn map_index(&self, index: u32) -> usize {
        if index < self.first_changed_index
            || (self.first_changed_index == 0 && self.last_changed_index == 0)
        {
            0
        } else if index >= self.last_changed_index {
            (self.last_changed_index - self.first_changed_index + 1) as usize
        } else {
            (index - self.first_changed_index + 1) as usize
        }
    }
}
