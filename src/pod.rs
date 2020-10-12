use crate::result::*;
use std::convert::TryFrom;
pub use std::convert::TryInto;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PodType {
    Boolean = 0,
    U8,
    I8,

    U16,
    I16,

    U32,
    I32,

    U64,
    I64,

    F16,
    F32,
    F64,

    String,
    WString,

    Unknown = 127,
}

#[derive(Debug)]
pub enum PodArray {
    Boolean(Vec<bool>),
    U8(Vec<u8>),
    I8(Vec<i8>),

    U16(Vec<u16>),
    I16(Vec<i16>),

    U32(Vec<u32>),
    I32(Vec<i32>),

    U64(Vec<u64>),
    I64(Vec<i64>),

    F16(Vec<half::f16>),
    F32(Vec<f32>),
    F64(Vec<f64>),

    String(Vec<String>),
    WString(Vec<String>),
}

impl TryFrom<u32> for PodType {
    type Error = OgawaError;

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == PodType::Boolean as u32 => Ok(PodType::Boolean),
            x if x == PodType::U8 as u32 => Ok(PodType::U8),
            x if x == PodType::I8 as u32 => Ok(PodType::I8),
            x if x == PodType::U16 as u32 => Ok(PodType::U16),
            x if x == PodType::I16 as u32 => Ok(PodType::I16),
            x if x == PodType::U32 as u32 => Ok(PodType::U32),
            x if x == PodType::I32 as u32 => Ok(PodType::I32),
            x if x == PodType::U64 as u32 => Ok(PodType::U64),
            x if x == PodType::I64 as u32 => Ok(PodType::I64),

            x if x == PodType::F16 as u32 => Ok(PodType::F16),
            x if x == PodType::F32 as u32 => Ok(PodType::F32),
            x if x == PodType::F64 as u32 => Ok(PodType::F64),

            x if x == PodType::String as u32 => Ok(PodType::String),
            x if x == PodType::WString as u32 => Ok(PodType::WString),

            x if x == PodType::Unknown as u32 => Ok(PodType::Unknown),

            _ => Err(ParsingError::UnsupportedAlembicFile.into()),
        }
    }
}
#[derive(Debug, Clone)]
pub struct DataType {
    pub(crate) pod_type: PodType,
    pub(crate) extent: u8,
}
