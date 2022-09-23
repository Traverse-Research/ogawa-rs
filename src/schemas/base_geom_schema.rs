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
            .load_sub_property(0, reader, archive)?
            .try_into()?;
        Self::new_from_properties(&properties, reader, archive)
    }

    pub fn new_from_properties(
        properties: &CompoundPropertyReader,
        reader: &mut dyn ArchiveReader,
        archive: &Archive,
    ) -> Result<Self> {
        let self_bounds: ScalarPropertyReader = properties
            .load_sub_property_by_name(".selfBnds", reader, archive)?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;
        if self_bounds.header.data_type != BOX_TYPE {
            return Err(ParsingError::IncompatibleSchema.into());
        }

        Ok(Self { self_bounds })
    }

    pub fn load_bounds_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<BoundingBox> {
        let pod_array = self.self_bounds.load_sample(sample_index, reader)?;
        let pod_array = if let PodArray::F64(array) = pod_array {
            array
        } else {
            return Err(InternalError::Unreachable.into());
        };

        Ok(BoundingBox {
            min: [pod_array[0], pod_array[1], pod_array[2]],
            max: [pod_array[3], pod_array[4], pod_array[5]],
        })
    }
}
