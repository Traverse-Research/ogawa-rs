use ogawa_rs::*;
fn main() -> ogawa_rs::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        return Err(ogawa_rs::OgawaError::Other(anyhow::anyhow!(
            "Missing required filename argument."
        )));
    }

    let mut reader = MemMappedReader::new(&args[1])?;
    // let mut reader = FileReader::new(&args[1])?;

    let archive = Archive::new(&mut reader)?;

    let mut stack = vec![archive.load_root_object(&mut reader)?];
    loop {
        if stack.is_empty() {
            break;
        }

        let current = stack.pop().unwrap();

        match Schema::parse(&current, &mut reader, &archive) {
            Ok(schema) => match &schema {
                Schema::BaseGeom(_) => println!("base geometry schema."),
                Schema::Curves(curves) => {
                    println!("curves schema.");
                    println!("\tcurves.is_constant() -> {}", curves.is_constant());
                    println!(
                        "\tcurves.topology_variance() -> {:?}",
                        curves.topology_variance()
                    );
                    println!(
                        "\tcurves.has_position_weights() -> {}",
                        curves.has_position_weights()
                    );
                    println!("\tcurves.has_uv() -> {}", curves.has_uv());
                    println!("\tcurves.has_n() ->: {}", curves.has_n());
                    println!("\tcurves.has_width() -> {}", curves.has_width());
                    println!("\tcurves.has_velocities() -> {}", curves.has_velocities());
                    println!("\tcurves.has_orders() -> {}", curves.has_orders());
                    println!("\tcurves.has_knots() -> {}", curves.has_knots());

                    let (curve_type, curve_periodicity, basis_type) =
                        curves.load_curve_type_sample(0, &mut reader)?;
                    println!("\tcurve type: {:?}", curve_type);
                    println!("\tcurve periodicity: {:?}", curve_periodicity);
                    println!("\tbasis type: {:?}", basis_type);

                    let positions = curves.load_positions_sample(0, &mut reader)?;
                    println!("\tnumber of positions: {}", positions.len());

                    let n_vertices = curves.load_n_vertices_sample(0, &mut reader)?;
                    println!("\tnumber of curves: {}", n_vertices.len());

                    println!(
                        "\taverage number of points per curve: {}",
                        positions.len() as f32 / n_vertices.len() as f32
                    );
                }
                Schema::Xform(xform) => {
                    println!("xform.is_constant() -> {}", xform.is_constant());
                    println!(
                        "\txform.is_constant_identity() -> {}",
                        xform.is_constant_identity()
                    );
                }
            },
            Err(err) => match err {
                OgawaError::ParsingError(ParsingError::IncompatibleSchema) => {
                    println!("no compatible schema")
                }
                _ => return Err(err),
            },
        }

        let child_count = current.child_count();
        for i in (0..child_count).rev() {
            let child = current.load_child(
                i,
                &mut reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?;

            stack.push(child);
        }
    }
    Ok(())
}
