use ogawa_rs::*;

fn print_chunk_tree(root_group: &GroupChunk, reader: &mut dyn ArchiveReader) -> Result<()> {
    let mut total_data_size = 0;
    let mut data_count = 0;
    let mut group_count = 0;
    let mut stack = vec![(0, 0, Chunk::Group(root_group.clone()))];

    loop {
        if stack.is_empty() {
            break;
        }

        let (indent, index, current) = stack.pop().unwrap();
        group_count += 1;

        for _ in 0..indent {
            print!("|   ");
        }

        match &current {
            Chunk::Group(group) => {
                println!(
                    "({}) group: 0x{:016x} ({} children)",
                    index, group.position, group.child_count
                );
            }
            Chunk::Data(data) => {
                println!(
                    "({}) data: 0x{:016x} ({} bytes)",
                    index, data.position, data.size
                );
            }
        }

        if let Chunk::Group(current_group) = &current {
            for (i, &child) in current_group.children.iter().enumerate().rev() {
                if is_group(child) {
                    let group = current_group.load_group(reader, i, false)?;
                    stack.push((indent + 1, i, Chunk::Group(group)));
                } else {
                    let data = current_group.load_data(reader, i)?;

                    total_data_size += data.size;
                    data_count += 1;

                    stack.push((indent + 1, i, Chunk::Data(data)));
                }
            }
        }
    }

    println!("total_data_size: {}", total_data_size);
    println!("data_count: {}", data_count);
    println!("group_count: {}", group_count);
    Ok(())
}

fn print_object_structure(reader: &mut dyn ArchiveReader, archive: &Archive) -> Result<()> {
    let object_reader = archive.load_root_object(reader)?;

    let mut stack = vec![(0, object_reader)];

    loop {
        if stack.is_empty() {
            break;
        }

        let (indent, current) = stack.pop().unwrap();
        let header = current.header.clone();
        for _ in 0..indent {
            print!("|   ");
        }
        println!(
            "object: {} (metadata: {})",
            &header.full_name,
            header.meta_data.serialize()
        );

        let child_count = current.child_map.len();
        for i in 0..child_count {
            let child = current.load_child(
                i,
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?;
            stack.push((indent + 1, child));
        }

        let properties = current.properties().unwrap();
        let mut prop_stack = vec![];
        for i in (0..properties.sub_property_count()).rev() {
            let prop = properties.load_sub_property(
                i,
                reader,
                &archive.indexed_meta_data,
                &archive.time_samplings,
            )?;
            prop_stack.push((1, prop));
        }

        loop {
            if prop_stack.is_empty() {
                break;
            }

            let (prop_indent, properties) = prop_stack.pop().unwrap();

            if let PropertyReader::Compound(properties) = &properties {
                for i in (0..properties.sub_property_count()).rev() {
                    let prop = properties.load_sub_property(
                        i,
                        reader,
                        &archive.indexed_meta_data,
                        &archive.time_samplings,
                    )?;

                    prop_stack.push(((prop_indent + 1), prop));
                }
            }

            let prop_name = properties.name().to_owned();
            let (typename, metadata) = match &properties {
                PropertyReader::Array(r) => ("array", r.header.meta_data.serialize()),
                PropertyReader::Compound(r) => ("compound", r.header.meta_data.serialize()),
                PropertyReader::Scalar(r) => ("scalar", r.header.meta_data.serialize()),
            };
            for _ in 0..(indent + prop_indent) {
                print!("|   ");
            }
            println!(
                "prop(type: {}, metadata: {}): {}",
                typename, metadata, prop_name
            );

            match &properties {
                PropertyReader::Scalar(pr) => {
                    for i in 0..pr.sample_count() {
                        for _ in 0..(indent + prop_indent + 1) {
                            print!("|   ");
                        }
                        let size = pr.sample_size(i, reader)?;
                        print!("scalar data {:?} ({} bytes)", &pr.header.data_type, size);
                        let sample = pr.load_sample(i, reader)?;
                        print!("{:?}", &sample);
                        println!();
                    }
                }
                PropertyReader::Array(pr) => {
                    for i in 0..pr.sample_count() {
                        for _ in 0..(indent + prop_indent + 1) {
                            print!("|   ");
                        }
                        let size = pr.sample_size(i, reader)?;
                        print!("array data {:?} ({} bytes)", &pr.header.data_type, size);
                        let _sample = pr.load_sample(i, reader)?;
                        // print!("{:?}", &sample.len());
                        println!();
                    }
                }
                PropertyReader::Compound(_) => {}
            }
        }
    }

    Ok(())
}

fn main() -> ogawa_rs::Result<()> {
    let filepath = "test_assets/Eyelashes01.abc";

    let mut reader = MemMappedReader::new(filepath)?;
    // let mut reader = FileReader::new(filepath)?;

    let archive = Archive::new(&mut reader)?;

    println!("------ print_chunk_tree ------");
    print_chunk_tree(&archive.root_group, &mut reader)?;

    println!("------ print_object_structure ------");
    print_object_structure(&mut reader, &archive)?;

    Ok(())
}
