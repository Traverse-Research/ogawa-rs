use ogawa_rs::*;
fn main() -> ogawa_rs::Result<()> {
    let filepath = "test_assets/Eyelashes01.abc";

    let mut reader = MemMappedReader::new(filepath)?;
    // let mut reader = FileReader::new(filepath)?;

    let archive = Archive::new(&mut reader)?;

    let object = archive.load_root_object(&mut reader)?;
    println!("object: {}", &object.header.full_name);

    let object = object.load_child(
        0,
        &mut reader,
        &archive.indexed_meta_data,
        &archive.time_samplings,
    )?;
    println!("object: {}", &object.header.full_name);

    let object = object.load_child(
        0,
        &mut reader,
        &archive.indexed_meta_data,
        &archive.time_samplings,
    )?;
    println!("object: {}", &object.header.full_name);

    let schema = CurvesSchema::new_from_object_reader(&object, &mut reader, &archive)?;

    let positions = schema.load_positions_sample(0, &mut reader)?;
    let n_vertices = schema.load_n_vertices_sample(0, &mut reader)?;

    let mut index = 0;
    for &line_len in n_vertices.iter() {
        let line_len = line_len as usize;
        let _line = &positions[index..(index + line_len)];
        //println!("line[{}]: {:?}", line_len, _line);
        index += line_len;
    }
    assert!(index == positions.len());

    println!("has_uv: {}", schema.has_uv());
    println!("has_n: {}", schema.has_n());
    println!("has_width: {}", schema.has_width());
    println!("has_velocities: {}", schema.has_velocities());
    println!("has_orders: {}", schema.has_orders());
    println!("has_knots: {}", schema.has_knots());

    dbg!(schema);

    Ok(())
}
