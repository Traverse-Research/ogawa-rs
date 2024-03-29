use super::base_geom_schema::BaseGeomSchema;
use crate::object_reader::ObjectReader;
use crate::pod::*;
use crate::property::*;
use crate::reader::ArchiveReader;
use crate::result::*;
use crate::Archive;
use std::convert::TryFrom;
pub use std::convert::TryInto;
#[derive(Debug, PartialEq, Eq)]
pub enum TopologyVariance {
    ConstantTopology,
    HomogeneousTopology,
    HeterogeneousTopology,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CurvePeriodicity {
    NonPeriodic = 0,
    Periodic = 1,
}
impl TryFrom<u8> for CurvePeriodicity {
    type Error = ParsingError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == CurvePeriodicity::NonPeriodic as u8 => Ok(CurvePeriodicity::NonPeriodic),
            x if x == CurvePeriodicity::Periodic as u8 => Ok(CurvePeriodicity::Periodic),

            _ => Err(ParsingError::InvalidAlembicFile),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CurveType {
    Cubic = 0,
    Linear = 1,
    VariableOrder = 2,
}
impl TryFrom<u8> for CurveType {
    type Error = ParsingError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == CurveType::Cubic as u8 => Ok(CurveType::Cubic),
            x if x == CurveType::Linear as u8 => Ok(CurveType::Linear),
            x if x == CurveType::VariableOrder as u8 => Ok(CurveType::VariableOrder),

            _ => Err(ParsingError::InvalidAlembicFile),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BasisType {
    None = 0,
    Bezier = 1,
    Bspline = 2,
    Catmullrom = 3,
    Hermite = 4,
    Power = 5,
}
impl TryFrom<u8> for BasisType {
    type Error = ParsingError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == BasisType::None as u8 => Ok(BasisType::None),
            x if x == BasisType::Bezier as u8 => Ok(BasisType::Bezier),
            x if x == BasisType::Bspline as u8 => Ok(BasisType::Bspline),
            x if x == BasisType::Catmullrom as u8 => Ok(BasisType::Catmullrom),
            x if x == BasisType::Hermite as u8 => Ok(BasisType::Hermite),
            x if x == BasisType::Power as u8 => Ok(BasisType::Power),

            _ => Err(ParsingError::InvalidAlembicFile),
        }
    }
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
            .load_sub_property(0, reader, archive)?
            .try_into()?;

        let base_geom = BaseGeomSchema::new_from_properties(&properties, reader, archive)?;

        // load required properties
        let positions: ArrayPropertyReader = properties
            .load_sub_property_by_name_checked("P", reader, archive, Some(&F32X3_TYPE))?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;
        let n_vertices: ArrayPropertyReader = properties
            .load_sub_property_by_name_checked("nVertices", reader, archive, Some(&I32_TYPE))?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;
        let curve_basis_and_type: ScalarPropertyReader = properties
            .load_sub_property_by_name_checked("curveBasisAndType", reader, archive, None)?
            .ok_or(ParsingError::IncompatibleSchema)?
            .try_into()?;

        // load optional properties
        let position_weights = properties
            .load_sub_property_by_name_checked("w", reader, archive, Some(&F32_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let uv = properties
            .load_sub_property_by_name_checked("uv", reader, archive, Some(&F32X2_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let n = properties
            .load_sub_property_by_name_checked("n", reader, archive, Some(&F32X3_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let width = properties
            .load_sub_property_by_name_checked("width", reader, archive, Some(&F32_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let velocities = properties
            .load_sub_property_by_name_checked(".velocities", reader, archive, Some(&F32X3_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let orders = properties
            .load_sub_property_by_name_checked(".orders", reader, archive, Some(&U8_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;
        let knots = properties
            .load_sub_property_by_name_checked(".knots", reader, archive, Some(&F32_TYPE))?
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

    pub fn load_bounds_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<BoundingBox> {
        self.base_geom.load_bounds_sample(sample_index, reader)
    }

    pub fn load_curve_type_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<(CurveType, CurvePeriodicity, BasisType)> {
        let pod_array = self
            .curve_basis_and_type
            .load_sample(sample_index, reader)?;
        let pod_array = if let PodArray::U8(array) = pod_array {
            array
        } else {
            return Err(InternalError::Unreachable.into());
        };

        let curve_type = (*pod_array.first().ok_or(InternalError::Unreachable)?).try_into()?;
        let periodicity = (*pod_array.get(1).ok_or(InternalError::Unreachable)?).try_into()?;
        let basis_type = (*pod_array.get(2).ok_or(InternalError::Unreachable)?).try_into()?;

        Ok((curve_type, periodicity, basis_type))
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

    pub fn load_widths_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Option<Vec<f32>>> {
        if let Some(width) = &self.width {
            let array = width.load_sample(sample_index, reader)?;
            if let PodArray::F32(array) = array {
                Ok(Some(array))
            } else {
                Err(InternalError::Unreachable.into())
            }
        } else {
            Ok(None)
        }
    }

    pub fn load_velocities_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Option<Vec<[f32; 3]>>> {
        if let Some(velocities) = &self.velocities {
            let array = velocities.load_sample(sample_index, reader)?;
            if let PodArray::F32(array) = array {
                Ok(Some(chunk_vector_by_3(array)?))
            } else {
                Err(InternalError::Unreachable.into())
            }
        } else {
            Ok(None)
        }
    }

    pub fn load_orders_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Option<Vec<u8>>> {
        if let Some(orders) = &self.orders {
            let array = orders.load_sample(sample_index, reader)?;
            if let PodArray::U8(array) = array {
                Ok(Some(array))
            } else {
                Err(InternalError::Unreachable.into())
            }
        } else {
            Ok(None)
        }
    }

    pub fn load_knots_sample(
        &self,
        sample_index: u32,
        reader: &mut dyn ArchiveReader,
    ) -> Result<Option<Vec<f32>>> {
        if let Some(knots) = &self.knots {
            let array = knots.load_sample(sample_index, reader)?;
            if let PodArray::F32(array) = array {
                Ok(Some(array))
            } else {
                Err(InternalError::Unreachable.into())
            }
        } else {
            Ok(None)
        }
    }
}
