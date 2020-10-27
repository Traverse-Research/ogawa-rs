use crate::object_reader::ObjectReader;
use crate::pod::*;
use crate::property::*;
use crate::reader::ArchiveReader;
use crate::result::*;
use crate::Archive;
#[derive(Debug)]
pub struct BaseGeomSchema {
    self_bounds: ScalarPropertyReader,
}
impl BaseGeomSchema {
    pub fn new_from_object_reader(
        object: &ObjectReader,
        reader: &mut dyn ArchiveReader,
        archive: &Archive,
    ) -> Result<Self> {
        let properties = object
            .properties()
            .ok_or(ParsingError::IncompatibleSchema)?;
        let properties: CompoundPropertyReader = properties
            .load_sub_property(0, reader, &archive)?
            .try_into()?;
        Self::new_from_properties(&properties, reader, archive)
    }

    pub fn new_from_properties(
        properties: &CompoundPropertyReader,
        reader: &mut dyn ArchiveReader,
        archive: &Archive,
    ) -> Result<Self> {
        let self_bounds: ScalarPropertyReader = properties
            .load_sub_property_by_name(".selfBnds", reader, &archive)?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;
        if self_bounds.header.data_type != BOX_TYPE {
            return Err(ParsingError::IncompatibleSchema.into());
        }

        Ok(Self { self_bounds })
    }
}
