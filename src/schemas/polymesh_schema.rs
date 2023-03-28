use super::base_geom_schema::BaseGeomSchema;
use crate::object_reader::ObjectReader;
use crate::pod::*;
use crate::property::*;
use crate::reader::ArchiveReader;
use crate::result::*;
use crate::Archive;
pub use std::convert::TryInto;

#[derive(Debug)]
pub struct PolyMeshSchema {
    base_geom: BaseGeomSchema,
    pub facecounts: ArrayPropertyReader,
    pub faceindices: ArrayPropertyReader,
    pub vertices: ArrayPropertyReader,
    pub normals: Option<ArrayPropertyReader>,
    pub uv: Option<CompoundPropertyReader>,
    pub velocities: Option<ArrayPropertyReader>
}

impl PolyMeshSchema {
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
        
        let base_geom = BaseGeomSchema::new_from_properties(&properties, reader, archive)?;

        // load required properties
        let vertices: ArrayPropertyReader = properties
            .load_sub_property_by_name_checked("P", reader, archive, Some(&F32X3_TYPE))?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;
        
        let faceindices: ArrayPropertyReader = properties
            .load_sub_property_by_name_checked(".faceIndices", reader, archive, Some(&I32_TYPE))?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;
        
        let facecounts: ArrayPropertyReader = properties
        .load_sub_property_by_name_checked(".faceCounts", reader, archive, Some(&I32_TYPE))?
        .ok_or(ParsingError::IncompatibleSchema)?
        .try_into()?;        

        // load optional properties
        let normals = properties
            .load_sub_property_by_name_checked("N", reader, archive, Some(&F32X3_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        
        let uv = properties
            .load_sub_property_by_name("uv", reader, archive)?
            .map(|x| -> Result<CompoundPropertyReader> { Ok(x.try_into()?) })
            .transpose()?;
        
        let velocities = properties
            .load_sub_property_by_name_checked("velocities", reader, archive, Some(&F32X3_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;

        Ok(Self {
            base_geom,
            facecounts,
            faceindices,
            vertices,
            normals,
            uv,
            velocities
        })
    }


    pub fn has_normals(&self) -> bool {
        self.normals.is_some()
    }

    pub fn has_uv(&self) -> bool {
        self.uv.is_some()
    }

    pub fn has_velocities(&self) -> bool {
        self.velocities.is_some()
    }


    pub fn load_bounds_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<BoundingBox> {
        self.base_geom.load_bounds_sample(sample_index, reader)
    }

    pub fn load_vertices_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Vec<[f32; 3]>> {
        let pod_array = self.vertices.load_sample(sample_index, reader)?;
        let pod_array = if let PodArray::F32(array) = pod_array {
            array
        } else {
            return Err(InternalError::Unreachable.into());
        };
        
        Ok(chunk_vector_by_3(pod_array)?)
    }

    pub fn load_facecounts_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Vec<i32>> {
        let pod_array = self.facecounts.load_sample(sample_index, reader)?;
        if let PodArray::I32(array) = pod_array {
            Ok(array)
        } else {
            Err(InternalError::Unreachable.into())
        }
    }

    pub fn load_faceindices_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Vec<i32>> {
        let pod_array = self.faceindices.load_sample(sample_index, reader)?;
        if let PodArray::I32(array) = pod_array {
            Ok(array)
        } else {
            Err(InternalError::Unreachable.into())
        }
    }
}
