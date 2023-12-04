use crate::object_reader::ObjectReader;
use crate::pod::*;
use crate::property::*;
use crate::reader::ArchiveReader;
use crate::result::*;
use crate::Archive;

#[derive(Debug)]
pub struct XformSchema {
    pub child_bounds: Option<ScalarPropertyReader>,
    pub arb_geometry_parameters: Option<CompoundPropertyReader>,
    pub user_properties: Option<CompoundPropertyReader>,
    pub is_constant_identity: bool,
    pub is_constant: bool,
}

impl XformSchema {
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

        let child_bounds = properties
            .load_sub_property_by_name_checked(".childBnds", reader, archive, Some(&BOX_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;

        let inherits: Option<ScalarPropertyReader> = properties
            .load_sub_property_by_name_checked(".inherits", reader, archive, Some(&BOOL_TYPE))?
            .map(|x| x.try_into())
            .transpose()?;

        let vals = properties
            .load_sub_property_by_name(".vals", reader, archive)?
            .map(|x| {
                let _data_type = match &x {
                    PropertyReader::Array(r) => &r.header.data_type,
                    PropertyReader::Scalar(r) => &r.header.data_type,
                    _ => return Err(ParsingError::IncompatibleSchema),
                };

                //TODO(max): What data type should we check for?
                //if data_type.pod_type != PodType::

                Ok(x)
            })
            .transpose()?;

        let is_constant_identity = properties
            .find_sub_property_index("isNotConstantIdentity")
            .is_none();

        let is_constant = if let Some(vals) = &vals {
            match vals {
                PropertyReader::Array(r) => r.is_constant(),
                PropertyReader::Scalar(r) => r.is_constant(),
                _ => return Err(ParsingError::IncompatibleSchema.into()),
            }
        } else {
            true
        };

        let is_constant = is_constant
            && if let Some(inherits) = &inherits {
                inherits.is_constant()
            } else {
                true
            };

        // TODO(max): Animation channels
        /*
        let mut anim_channels = std::collections::BTreeSet::<u32>::new();
        let anim_channels_prop = properties
            .load_sub_property_by_name(
                ".animChans",
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?
            .map(|x| {
                let x: ArrayPropertyReader = x.try_into()?;
                if x.header.data_type == BOX_TYPE {
                    Ok(x)
                } else {
                    Err(ParsingError::IncompatibleSchema)
                }
            })
            .transpose()?;
        if let Some(anim_channels_prop) = anim_channels_prop {
            let sample_count = anim_channels_prop.sample_count();
            if sample_count > 0 {
                for i in 0..sample_count {

                }
            }
        }
        */

        // TODO(max): ops

        let arb_geometry_parameters = properties
            .load_sub_property_by_name(".arbGeomParams", reader, archive)?
            .map(|x| x.try_into())
            .transpose()?;

        let user_properties = properties
            .load_sub_property_by_name(".userProperties", reader, archive)?
            .map(|x| x.try_into())
            .transpose()?;

        Ok(Self {
            child_bounds,
            is_constant_identity,
            is_constant,
            arb_geometry_parameters,
            user_properties,
        })
    }

    pub fn is_constant(&self) -> bool {
        self.is_constant
    }
    pub fn is_constant_identity(&self) -> bool {
        self.is_constant_identity
    }
}
