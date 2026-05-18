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
    Help,
    Clear,
    Worldgen(bool),
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
        ["help"] => Some(CliCommand::Help),
        ["clear"] => Some(CliCommand::Clear),
        ["worldgen", on] => Some(CliCommand::Worldgen(
            if on.to_lowercase() == "on" {true}
            else if on.to_lowercase() == "off" {false}
            else {return None}
        )),
        _ => None,
    }
}

pub fn execute_cli_commands(app: &mut App, event_loop: &ActiveEventLoop, cmd: CliCommand){
    match cmd {
        CliCommand::Quit => event_loop.exit(),
        CliCommand::Parse { path, min_width } => {
            println!("Parsing file to readable format...");
            let mut mesh = match file_parser::file_parse_interface(&path) {
                Ok(mesh) => mesh,
                Err(e) => {
                    println!("{}, please try again", e);
                    return;
                },
            };

            let color_offset = app.colours.len();

            for face in &mut mesh.faces {
                face.color_id += color_offset;
            }

            for color in &mesh.colors {
                app.colours.push([color.x, color.y, color.z, 0.0]);
            }

            println!("Translating points to voxel geometry...");
            let world_data = voxelizer::voxel_grid_from_triangles(mesh, min_width);

            println!("Compressing world into Sparse Voxel Octrees...");
            let chunks = to_chunks(&world_data);

            println!("Successfully built {} chunks!", chunks.len());

            // Add to existing chunk or insert new if no chunk exists
            for (key, value) in chunks {
                if let Some(chunk) = app.chunks.get_mut(&key) {
                    chunk.add_chunk(&value);
                } else {
                    app.chunks.insert(key, value);
                }
            }
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

                    // Add to existing chunk or insert new if no chunk exists
                    for (key, value) in data {
                        if let Some(chunk) = app.chunks.get_mut(&key) {
                            chunk.add_chunk(&value);
                        } else {
                            app.chunks.insert(key, value);
                        }
                    }
                  
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
        },
        CliCommand::Help => {
            print_help();

        },
        CliCommand::Clear => {
            app.chunks.clear();
            reset_and_upload_world(app);            
        }
        CliCommand::Worldgen(on) => {
            // Skip if state not changed
            if app.use_worldgen == on {
                return;
            }

            // Clear if off
            if !on {
                app.worldgen_chunks.clear();
            }

            app.use_worldgen = on;
            reset_and_upload_world(app);
        }
    }
}

fn print_help() {
    println!(r#"
============================================================
                      RAYCASTER HELP
============================================================
Available Commands:

  quit, exit
      Exits the application.

  parse <path> <min_width>
      Parses a 3D model file into the voxel grid.
      Example: parse models/house.obj 32

  save <path>
      Saves the current voxel world to a file.
      Example: save saves/world1.dat

  load <path>
      Loads a voxel world from a file.
      Example: load saves/world1.dat

  colors
      Prints the current color palette and their reflectivity.

  change_color <i> <r> <g> <b> <a>
      Changes a color. 'i' is the index (or 'a' for all, 'b' for all alpha/reflectivity).
      RGBA values must be between 0.0 and 1.0.
      Example: change_color 1 1.0 0.0 0.0 1.0

  time <time> <speed>
      Sets the time of day and the speed of the day/night cycle.
      Example: time 0.5 0.1

  clear
      Removes all loaded objects by clearing the chunks

  help
      Displays this menu.
============================================================
"#);
}

fn reset_and_upload_world(app: &mut App) {
    let state = app.state.as_mut().unwrap();
    
    state.active_chunks.clear();
    
    let indexer_size = state.grid_size * state.grid_size * state.grid_size;
    let max_u32_elements = (state.world_buffer.size() / 4) as u32;
    state.allocator = crate::VoxelHeapAllocator::new(indexer_size, max_u32_elements);
    
    let empty_indexer = vec![0xFFFFFFFFu32; indexer_size as usize];
    state.queue.write_buffer(&state.world_buffer, 0, bytemuck::cast_slice(&empty_indexer));

    app.world_changed = true;
}
