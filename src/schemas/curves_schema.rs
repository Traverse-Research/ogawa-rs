use crate::object_reader::ObjectReader;
use crate::pod::*;
use crate::property::*;
use crate::reader::ArchiveReader;
use crate::result::*;
use crate::Archive;

#[derive(Debug)]
pub struct CurvesSchema {
    p: ArrayPropertyReader,
    n_vertices: ArrayPropertyReader,
    curve_basis_and_type: ScalarPropertyReader,

    uv: Option<ArrayPropertyReader>,
    n: Option<ArrayPropertyReader>,
    width: Option<ArrayPropertyReader>,
    velocities: Option<ArrayPropertyReader>,
    orders: Option<ArrayPropertyReader>,
    knots: Option<ArrayPropertyReader>,
}

impl CurvesSchema {
    pub fn new_from_object_reader(
        object: &ObjectReader,
        reader: &mut dyn ArchiveReader,
        archive: &Archive,
    ) -> Result<Self> {
        let properties = object
            .properties()
            .ok_or(ParsingError::IncompatibleSchema)?;
        let properties: CompoundPropertyReader = properties
            .load_sub_property(
                0,
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .try_into()?;

        println!("attempting to parse P");
        let p: ArrayPropertyReader = properties
            .load_sub_property_by_name(
                "P",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;
        if p.header.data_type != F32X3_TYPE {
            return Err(ParsingError::IncompatibleSchema.into());
        }

        println!("attempting to parse nVertices");
        let n_vertices: ArrayPropertyReader = properties
            .load_sub_property_by_name(
                "nVertices",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;
        if n_vertices.header.data_type != I32_TYPE {
            return Err(ParsingError::IncompatibleSchema.into());
        }

        println!("attempting to parse curveBasisAndType");
        let curve_basis_and_type: ScalarPropertyReader = properties
            .load_sub_property_by_name(
                "curveBasisAndType",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;

        println!("attempting to parse uv");
        let uv = properties
            .load_sub_property_by_name(
                "uv",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .map(|x| {
                let x: ArrayPropertyReader = x.try_into()?;
                if x.header.data_type == F32X2_TYPE {
                    Ok(x)
                } else {
                    Err(ParsingError::IncompatibleSchema)
                }
            })
            .transpose()?;

        println!("attempting to parse N");
        let n = properties
            .load_sub_property_by_name(
                "N",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .map(|x| {
                let x: ArrayPropertyReader = x.try_into()?;
                if x.header.data_type == F32X3_TYPE {
                    Ok(x)
                } else {
                    Err(ParsingError::IncompatibleSchema)
                }
            })
            .transpose()?;

        println!("attempting to parse width");
        let width = properties
            .load_sub_property_by_name(
                "width",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .map(|x| {
                let x: ArrayPropertyReader = x.try_into()?;
                if x.header.data_type == F32_TYPE {
                    Ok(x)
                } else {
                    Err(ParsingError::IncompatibleSchema)
                }
            })
            .transpose()?;

        println!("attempting to parse velocities");
        let velocities = properties
            .load_sub_property_by_name(
                ".velocities",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .map(|x| {
                let x: ArrayPropertyReader = x.try_into()?;
                if x.header.data_type == F32X3_TYPE {
                    Ok(x)
                } else {
                    Err(ParsingError::IncompatibleSchema)
                }
            })
            .transpose()?;

        println!("attempting to parse .orders");
        let orders = properties
            .load_sub_property_by_name(
                ".orders",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .map(|x| {
                let x: ArrayPropertyReader = x.try_into()?;
                if x.header.data_type == U8_TYPE {
                    Ok(x)
                } else {
                    Err(ParsingError::IncompatibleSchema)
                }
            })
            .transpose()?;

        println!("attempting to parse .knots");
        let knots = properties
            .load_sub_property_by_name(
                ".knots",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .map(|x| {
                let x: ArrayPropertyReader = x.try_into()?;
                if x.header.data_type == F32_TYPE {
                    Ok(x)
                } else {
                    Err(ParsingError::IncompatibleSchema)
                }
            })
            .transpose()?;

        Ok(Self {
            p,
            n_vertices,
            curve_basis_and_type,
            uv,
            n,
            width,
            velocities,
            orders,
            knots,
        })
    }

    pub fn has_uv(&self) -> bool {
        self.uv.is_some()
    }
    pub fn has_n(&self) -> bool {
        self.n.is_some()
    }
    pub fn has_width(&self) -> bool {
        self.width.is_some()
    }
    pub fn has_velocities(&self) -> bool {
        self.velocities.is_some()
    }
    pub fn has_orders(&self) -> bool {
        self.orders.is_some()
    }
    pub fn has_knots(&self) -> bool {
        self.knots.is_some()
    }

    pub fn load_positions_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Vec<[f32; 3]>> {
        let pod_array = self.p.load_sample(sample_index, reader)?;
        let pod_array = if let PodArray::F32(array) = pod_array {
            array
        } else {
            return Err(InternalError::Unreachable.into());
        };

        Ok(chunk_vector_by_3(pod_array)?)
    }

    pub fn load_n_vertices_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Vec<i32>> {
        let pod_array = self.n_vertices.load_sample(sample_index, reader)?;
        if let PodArray::I32(array) = pod_array {
            Ok(array)
        } else {
            Err(InternalError::Unreachable.into())
        }
    }

    pub fn load_curve_basis_and_type_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<[u32; 4]> {
        let pod_array = self
            .curve_basis_and_type
            .load_sample(sample_index, reader)?;
        let pod_array = if let PodArray::U32(array) = pod_array {
            array
        } else {
            return Err(InternalError::Unreachable.into());
        };

        if pod_array.len() != 4 {
            return Err(ParsingError::InvalidAlembicFile.into());
        }

        let mut slice = [0; 4];
        slice.copy_from_slice(&pod_array);

        Ok(slice)
    }

    pub fn load_uv_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Option<Vec<[f32; 2]>>> {
        if let Some(uv) = &self.uv {
            let array = uv.load_sample(sample_index, reader)?;
            if let PodArray::F32(array) = array {
                Ok(Some(chunk_vector_by_2(array)?))
            } else {
                Err(InternalError::Unreachable.into())
            }
        } else {
            Ok(None)
        }
    }
}
