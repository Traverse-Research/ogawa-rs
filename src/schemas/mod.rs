mod base_geom_schema;
mod curves_schema;
mod xform_schema;

pub use base_geom_schema::BaseGeomSchema;
pub use curves_schema::CurvesSchema;
pub use xform_schema::XformSchema;

use crate::object_reader::ObjectReader;
use crate::reader::ArchiveReader;
use crate::result::*;
use crate::Archive;

use crate::property::*;

#[derive(Debug)]
pub enum Schema {
    BaseGeom(Box<BaseGeomSchema>),
    Curves(Box<CurvesSchema>),
    Xform(Box<XformSchema>),
}

impl Schema {
    pub fn parse(
        object: &ObjectReader,
        reader: &mut dyn ArchiveReader,
        archive: &Archive,
    ) -> Result<Schema> {
        assert!(
            object.header.meta_data.serialize()
                == object.properties().unwrap().header.meta_data.serialize()
        );

        let mut schema_type = object
            .header
            .meta_data
            .tokens
            .get("schema")
            .map(|x| x.to_owned());

        if schema_type.is_none() {
            if let Some(props) = object.properties() {
                if props.sub_property_count() >= 1 {
                    let cpr: CompoundPropertyReader = props
                        .load_sub_property(
                            0,
                            reader,
                            &archive.indexed_meta_data,
                            &archive.time_samplings,
                        )?
                        .try_into()?;
                    schema_type = cpr
                        .header
                        .meta_data
                        .tokens
                        .get("schema")
                        .map(|x| x.to_owned());
                }
            }
        }

        let schema_type = schema_type.ok_or(ParsingError::IncompatibleSchema)?;
        match schema_type.as_str() {
            "AbcGeom_Curve_v2" => Ok(Schema::Curves(Box::new(
                CurvesSchema::new_from_object_reader(object, reader, archive)?,
            ))),
            "AbcGeom_GeomBase_v1" => Ok(Schema::BaseGeom(Box::new(
                BaseGeomSchema::new_from_object_reader(object, reader, archive)?,
            ))),
            "AbcGeom_PolyMesh_v1" => {
                println!("AbcGeom_PolyMesh_v1 schema not yet implemented.");
                Err(ParsingError::UnsupportedAlembicFile.into())
            }
            "AbcGeom_SubD_v1" => {
                println!("AbcGeom_SubD_v1 schema not yet implemented.");
                Err(ParsingError::UnsupportedAlembicFile.into())
            }
            "AbcGeom_NuPatch_v2" => {
                println!("AbcGeom_NuPatch_v2 schema not yet implemented.");
                Err(ParsingError::UnsupportedAlembicFile.into())
            }
            "AbcGeom_FaceSet_v1" => {
                println!("AbcGeom_FaceSet_v1 schema not yet implemented.");
                Err(ParsingError::UnsupportedAlembicFile.into())
            }
            "AbcGeom_Points_v1" => {
                println!("AbcGeom_Points_v1 schema not yet implemented.");
                Err(ParsingError::UnsupportedAlembicFile.into())
            }
            "AbcGeom_Xform_v3" => Ok(Schema::Xform(Box::new(
                XformSchema::new_from_object_reader(object, reader, archive)?,
            ))),
            "AbcGeom_Light_v1" => {
                println!("AbcGeom_Light_v1 schema not yet implemented.");
                Err(ParsingError::UnsupportedAlembicFile.into())
            }
            "AbcGeom_Camera_v1" => {
                println!("AbcGeom_Camera_v1 schema not yet implemented.");
                Err(ParsingError::UnsupportedAlembicFile.into())
            }
            _ => Err(ParsingError::IncompatibleSchema.into()),
        }
    }
}
