use ogawa_rs::*;

const WINDOW_WIDTH: usize = 1280;
const WINDOW_HEIGHT: usize = 720;

struct Curves {
    positions: Vec<[f32; 3]>,
}

fn load_curves(filepath: &str) -> Result<Vec<Curves>> {
    println!("loading \"{}\".", filepath);

    let mut result = vec![];

    let mut reader = MemMappedReader::new(filepath)?;
    // let mut reader = FileReader::new(filepath)?;

    let archive = Archive::new(&mut reader)?;

    let mut stack = vec![archive.load_root_object(&mut reader)?];
    loop {
        if stack.is_empty() {
            break;
        }

        let current = stack.pop().unwrap();

        match Schema::parse(&current, &mut reader, &archive) {
            Ok(schema) => {
                if let Schema::Curves(curves) = schema {
                    let positions = curves.load_positions_sample(0, &mut reader)?;

                    result.push(Curves { positions });
                }
            }
            Err(OgawaError::ParsingError(ParsingError::IncompatibleSchema)) => {}
            Err(err) => return Err(err),
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

    Ok(result)
}

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    anyhow::ensure!(args.len() > 1, "Expecting one or more filename arguments.");

    println!("loading archives.");
    let curves_vec = args[1..]
        .iter()
        .map(|filepath| load_curves(filepath))
        .collect::<Result<Vec<_>>>()?;
    let curves_vec = curves_vec
        .iter()
        .flat_map(|curves| curves.iter())
        .collect::<Vec<_>>();

    println!("initializing display");

    let mut window = minifb::Window::new(
        "ogawa-rs/curves-test-visualization",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        minifb::WindowOptions::default(),
    )?;

    let mut camera_pos = glam::Vec3::new(0.0, 6.0, -5.0);
    let mut camera_rx = 0.0 * std::f32::consts::PI;
    let mut camera_ry = 0.0 * std::f32::consts::PI;

    let aspect_ratio = WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32;

    let mut last_time = std::time::Instant::now();

    loop {
        if !window.is_open() || window.is_key_released(minifb::Key::Escape) {
            break;
        }

        let mut buffer = vec![0u32; WINDOW_WIDTH * WINDOW_HEIGHT];

        let now = std::time::Instant::now();
        let delta = (now - last_time).as_secs_f32();
        last_time = now;

        if window.is_key_down(minifb::Key::Up) {
            camera_rx += 0.5 * delta;
        }
        if window.is_key_down(minifb::Key::Down) {
            camera_rx -= 0.5 * delta;
        }
        if window.is_key_down(minifb::Key::Left) {
            camera_ry += 0.5 * delta;
        }
        if window.is_key_down(minifb::Key::Right) {
            camera_ry -= 0.5 * delta;
        }

        let mut movement = glam::Vec3::ZERO;
        if window.is_key_down(minifb::Key::W) {
            movement.z += 1.0 * delta;
        }
        if window.is_key_down(minifb::Key::S) {
            movement.z -= 1.0 * delta;
        }
        if window.is_key_down(minifb::Key::A) {
            movement.x -= 1.0 * delta;
        }
        if window.is_key_down(minifb::Key::D) {
            movement.x += 1.0 * delta;
        }
        if window.is_key_down(minifb::Key::Q) {
            movement.y -= 1.0 * delta;
        }
        if window.is_key_down(minifb::Key::E) {
            movement.y += 1.0 * delta;
        }
        let rotation =
            glam::Quat::from_rotation_y(-camera_ry) * glam::Quat::from_rotation_x(-camera_rx);

        camera_pos += rotation * movement;

        let view_matrix = glam::Mat4::from_rotation_x(camera_rx)
            * glam::Mat4::from_rotation_y(camera_ry)
            * glam::Mat4::from_translation(-camera_pos);
        let projection_matrix = glam::Mat4::perspective_lh(1.0, aspect_ratio, 0.1, 100.0);

        let vp = projection_matrix * view_matrix;

        for Curves { positions } in curves_vec.iter() {
            for p in positions.iter() {
                let p = glam::Vec3::from_slice(p) / 25.0;
                let p = vp * p.extend(1.0);

                // frustum culling
                if p.x < -p.w || p.x > p.w || p.y < -p.w || p.y > p.w || p.z < -p.w || p.z > p.w {
                    continue;
                }
                let p = glam::vec3(p.x / p.w, p.y / p.w, p.z / p.w);

                //to screen coords
                let ss_x = (p.x * 0.5 + 0.5) * WINDOW_WIDTH as f32;
                let ss_y = (p.y * -0.5 + 0.5) * WINDOW_HEIGHT as f32;

                if ss_x >= 0.0
                    && ss_x < WINDOW_WIDTH as f32
                    && ss_y >= 0.0
                    && ss_y < WINDOW_HEIGHT as f32
                {
                    let ss_x = ss_x as usize;
                    let ss_y = ss_y as usize;

                    buffer[ss_x + ss_y * WINDOW_WIDTH] = 0xffff0000;
                }
            }
        }
        window.update_with_buffer(&buffer, WINDOW_WIDTH, WINDOW_HEIGHT)?;
    }

    Ok(())
}
