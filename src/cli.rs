use wgpu::util::DeviceExt;
use winit::event_loop::ActiveEventLoop;
use colored::Colorize;

use crate::{App, builder::{pack_world_to_gpu, to_chunks}, file_handler::{load_file_interface, save_file_interface}, file_parser, voxelizer};

#[derive(Debug)]
pub enum CliCommand {
    Quit,
    Parse{path: String, min_width: usize},
    Save(String),
    Load(String),
    PrintColors,
    ChangeColor{i: String, r: f32, g: f32, b: f32, a: f32}, 
    Time{time: f32, speed: f32},
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
        ["colors"] => Some(CliCommand::PrintColors),
        ["change_color", i, r,g,b,a] => Some(CliCommand::ChangeColor {
            i: i.to_string(),
            r: r.parse().ok()?,
            g: g.parse().ok()?,
            b: b.parse().ok()?,
            a: a.parse().ok()?,
        }),
        ["time", time, speed] => Some(CliCommand::Time {
            time: time.parse().ok()?,
            speed: speed.parse().ok()?,
        }),
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

            let mut colors: Vec<[f32; 4]> = Vec::new();

            for color in &mesh.colors {
                colors.push([color.x, color.y, color.z, 0.0]);
            }

            app.colours = colors;

            println!("Translating points to voxel geometry...");
            let world_data = voxelizer::voxel_grid_from_triangles(mesh, min_width);

            println!("Compressing world into Sparse Voxel Octrees...");
            let chunks = to_chunks(&world_data);

            println!("Successfully built {} chunks!", chunks.len());
            app.chunks = chunks;
            reset_and_upload_world(app);
        }
        CliCommand::Save(path) => {
            let data = &app.chunks;
            let colors = &app.colours;
            match save_file_interface(&path, data, colors) {
                Ok(_) => println!("Successfully saved data"),
                Err(e) => println!("{}, please try again", e),
            }
        },
        CliCommand::Load(path) => {
            match load_file_interface(&path) {
                Ok((data, colors)) => {
                    println!("Successfully loaded data");
                    app.chunks = data;
                  
                    //viktigt med färger, Alpha = reflectivity
                    app.colours = colors;
                  
                    reset_and_upload_world(app);

                },
                Err(e) => println!("{}, please try again", e),
            }
        },
        CliCommand::PrintColors => {

            println!("== Colors ==");
            for i in 0..app.colours.len() {
                let [r, g, b, a] = app.colours[i];

                let output = format!("Color, reflectivity: {}, {}", i, a);

                let r_u8 = (r * 255.0) as u8;
                let g_u8 = (g * 255.0) as u8;
                let b_u8 = (b * 255.0) as u8;                
                
                println!("{}", output.truecolor(r_u8, g_u8, b_u8));
            }
        },
        CliCommand::ChangeColor { i, r, g, b, a } => {
            if r <= 1.0 && g <= 1.0 && b <= 1.0 && a <= 1.0 && r >= 0.0 && g >= 0.0 && b >= 0.0 && a >= 0.0 { 
                
                match i.as_str() {
                    "a" => {
                        for color in app.colours.iter_mut() {
                            *color = [r, g, b, a];
                        }
                        println!("Successfully updated ALL colors.");
                    }
                    "b" => {
                        for color in app.colours.iter_mut() {
                            color[3] = a; 
                        }
                        println!("Successfully updated ALL reflectivity values to {}.", a);
                    }
                    _ => {
                        if let Ok(idx) = i.parse::<usize>() {
                            if idx < app.colours.len() {
                                app.colours[idx] = [r, g, b, a];
                                println!("Successfully updated color at index {}.", idx);
                            } else {
                                println!("Error: Index {} is out of bounds. Max index is {}.", idx, app.colours.len() - 1);
                                return;                            }
                        } else {
                            println!("Error: Invalid index '{}'. Use a number, 'a' (all), or 'b' (all alpha).", i);
                            return;
                        }
                    }
                }
                reset_and_upload_world(app);

            } else {
                println!("Error: All color values (r, g, b, a) must be between 0.0 and 1.0.");
            }
        },
        CliCommand::Time { time, speed} => {
            app.lighting.time = time;
            app.lighting.time_scale = speed;
        }
    }
}

fn reset_and_upload_world(app: &mut App) {
    let state = app.state.as_mut().unwrap();
    
    state.active_chunks.clear();
    
    let indexer_size = state.grid_size * state.grid_size * state.grid_size;
    let max_u32_elements = (state.world_buffer.size() / 4) as u32;
    state.allocator = crate::VoxelHeapAllocator::new(indexer_size, max_u32_elements);
    
    let empty_indexer = vec![0xFFFFFFFFu32; indexer_size as usize];
    state.queue.write_buffer(&state.world_buffer, 0, bytemuck::cast_slice(&empty_indexer));
}
