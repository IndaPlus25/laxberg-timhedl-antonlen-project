use wgpu::util::DeviceExt;
use winit::event_loop::ActiveEventLoop;

use crate::{App, builder::{pack_world_to_gpu, to_chunks}, file_handler::{load_file_interface, save_file_interface}, file_parser, voxelizer};

#[derive(Debug)]
pub enum CliCommand {
    Quit,
    Parse{path: String, min_width: usize},
    Save(String),
    Load(String),
}

pub fn parse_command(input: &str) -> Option<CliCommand> {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();

    match parts.as_slice() {
        ["quit"] | ["exit"] => Some(CliCommand::Quit),
        ["parse", path, x] => Some(CliCommand::Parse { 
            path: path.to_string(), 
            min_width: x.parse().ok()? 
        }),
        ["save", path] => Some(CliCommand::Save(path.to_string())),
        ["load", path] => Some(CliCommand::Load(path.to_string())),
        _ => None,
    }
}

pub fn execute_cli_commands(app: &mut App, event_loop: &ActiveEventLoop, cmd: CliCommand){
    match cmd {
        CliCommand::Quit => event_loop.exit(),
        CliCommand::Parse { path, min_width } => {
            println!("Parsing file to readable format...");
            let mesh = match file_parser::file_parse_interface(&path) {
                Ok(mesh) => mesh,
                Err(e) => {
                    println!("{}, please try again", e);
                    return;
                },
            };

            println!("Translating points to voxel geometry...");
            let world_data = voxelizer::voxel_grid_from_triangles(mesh, min_width);

            println!("Compressing world into Sparse Voxel Octrees...");
            let chunks = to_chunks(&world_data);

            println!("Successfully built {} chunks!", chunks.len());
            app.chunks = chunks;
            upload_world_to_gpu(app);
        }
        CliCommand::Save(path) => {
            let data = &app.chunks;
            match save_file_interface(&path, data) {
                Ok(_) => println!("Successfully saved data"),
                Err(e) => println!("{}, please try again", e),
            }
        },
        CliCommand::Load(path) => {
            match load_file_interface(&path) {
                Ok(data) => {
                    println!("Successfully loaded data");
                    app.chunks = data;
                    upload_world_to_gpu(app);
                },
                Err(e) => println!("{}, please try again", e),
            }
        },
    }
}

// Ill put this here for now but maybe move to main in future
fn upload_world_to_gpu(app: &mut App) {
    let packed = pack_world_to_gpu(&app.chunks, app.render_distance);
    let packed_bytes: &[u8] = bytemuck::cast_slice(&packed);

    let state = app.state.as_mut().unwrap();
    let existing_size = state.world_buffer.size();

    if packed_bytes.len() as u64 <= existing_size {
        state.queue.write_buffer(&state.world_buffer, 0, packed_bytes);
    } else {
        state.world_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("World Storage Buffer (Reloaded)"),
                contents: packed_bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }
        );
    }
}