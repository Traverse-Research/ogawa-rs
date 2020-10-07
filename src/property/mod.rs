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
