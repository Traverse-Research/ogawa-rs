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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PropertyType {
    Array,
    Compound,
    Scalar,
}

pub enum PropertyReader {
    Array(ArrayPropertyReader),
    Compound(CompoundPropertyReader),
    Scalar(ScalarPropertyReader),
}
impl PropertyReader {
    pub fn name(&self) -> &str {
        match self {
            Self::Array(r) => r.name(),
            Self::Compound(r) => r.name(),
            Self::Scalar(r) => r.name(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PropertyHeader {
    pub name: String,
    pub property_type: PropertyType,
    pub meta_data: MetaData,
    pub data_type: DataType,
    pub time_sampling: Option<Rc<TimeSampling>>,

    // friends?
    pub is_scalar_like: bool,
    pub is_homogenous: bool,
    pub next_sample_index: u32,
    pub first_changed_index: u32,
    pub last_changed_index: u32,
    pub time_sampling_index: u32,
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
