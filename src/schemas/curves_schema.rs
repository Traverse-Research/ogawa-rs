use super::base_geom_schema::BaseGeomSchema;
use crate::object_reader::ObjectReader;
use crate::pod::*;
use crate::property::*;
use crate::reader::ArchiveReader;
use crate::result::*;
use crate::Archive;

#[derive(Debug, PartialEq, Eq)]
pub enum TopologyVariance {
    ConstantTopology,
    HomogeneousTopology,
    HeterogeneousTopology,
}

#[derive(Debug)]
pub struct CurvesSchema {
    base_geom: BaseGeomSchema,

    positions: ArrayPropertyReader,
    n_vertices: ArrayPropertyReader,
    curve_basis_and_type: ScalarPropertyReader,

    position_weights: Option<ArrayPropertyReader>,
    uv: Option<ArrayPropertyReader>,
    n: Option<ArrayPropertyReader>,
    width: Option<ArrayPropertyReader>,
    velocities: Option<ArrayPropertyReader>,
    orders: Option<ArrayPropertyReader>,
    knots: Option<ArrayPropertyReader>,
}

fn load_sub_property(
    name: &str,
    properties: &CompoundPropertyReader,
    reader: &mut dyn ArchiveReader,
    archive: &Archive,
    data_type: Option<&DataType>,
) -> Result<Option<PropertyReader>> {
    let prop = properties
        .load_sub_property_by_name(
            name,
            reader,
            &archive.indexed_meta_data,
            &archive.time_samplings,
        )?
        .map(|prop| {
            if let Some(data_type) = data_type {
                let does_data_type_match = match &prop {
                    PropertyReader::Array(reader) => reader.header.data_type == *data_type,
                    PropertyReader::Scalar(reader) => reader.header.data_type == *data_type,
                    PropertyReader::Compound(reader) => reader.header.data_type == *data_type,
                };
                if !does_data_type_match {
                    return Err(ParsingError::IncompatibleSchema);
                }
            }
            Ok(prop)
        })
        .transpose()?;

    Ok(prop)
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

        let base_geom = BaseGeomSchema::new_from_properties(&properties, reader, archive)?;

        let positions: ArrayPropertyReader =
            load_sub_property("P", &properties, reader, archive, Some(&F32X3_TYPE))?
                .ok_or(ParsingError::IncompatibleSchema)?
                .try_into()?;
        let n_vertices: ArrayPropertyReader =
            load_sub_property("nVertices", &properties, reader, archive, Some(&I32_TYPE))?
                .ok_or(ParsingError::IncompatibleSchema)?
                .try_into()?;
        let curve_basis_and_type: ScalarPropertyReader =
            load_sub_property("curveBasisAndType", &properties, reader, archive, None)?
                .ok_or(ParsingError::IncompatibleSchema)?
                .try_into()?;

        let position_weights =
            load_sub_property("w", &properties, reader, archive, Some(&F32_TYPE))?
                .map(|x| x.try_into())
                .transpose()?;
        let uv = load_sub_property("uv", &properties, reader, archive, Some(&F32X2_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let n = load_sub_property("n", &properties, reader, archive, Some(&F32X3_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let width = load_sub_property("width", &properties, reader, archive, Some(&F32_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let velocities = load_sub_property(
            ".velocities",
            &properties,
            reader,
            archive,
            Some(&F32X3_TYPE),
        )?
        .map(|x| x.try_into())
        .transpose()?;
        let orders = load_sub_property(".orders", &properties, reader, archive, Some(&U8_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let knots = load_sub_property(".knots", &properties, reader, archive, Some(&F32_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;

        Ok(Self {
            base_geom,
            positions,
            n_vertices,
            curve_basis_and_type,
            position_weights,
            uv,
            n,
            width,
            velocities,
            orders,
            knots,
        })
    }

    pub fn topology_variance(&self) -> TopologyVariance {
        if self.n_vertices.is_constant() && self.curve_basis_and_type.is_constant() {
            let is_points_constant = self.positions.is_constant()
                && if let Some(w) = &self.position_weights {
                    w.is_constant()
                } else {
                    true
                };
            if is_points_constant {
                TopologyVariance::ConstantTopology
            } else {
                TopologyVariance::HomogeneousTopology
            }
        } else {
            TopologyVariance::HeterogeneousTopology
        }
    }
    pub fn is_constant(&self) -> bool {
        self.topology_variance() == TopologyVariance::ConstantTopology
    }
    pub fn has_position_weights(&self) -> bool {
        self.position_weights.is_some()
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
        let pod_array = self.positions.load_sample(sample_index, reader)?;
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
