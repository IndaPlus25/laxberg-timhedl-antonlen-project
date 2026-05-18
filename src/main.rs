mod file_parser;
mod voxelizer;

mod error;
mod file_handler;

mod octree;
mod vecmath;
mod builder;
mod worldgen;
mod renderer;
mod cli;
mod state;

use std::{collections::HashMap, f32::consts::FRAC_2_PI};
use std::rc::Rc;
use std::io::BufRead;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowId},
};
use std::time::Instant; 
use std::sync::Arc;
use wgpu::util::DeviceExt;
use colored::Colorize;

use crate::file_handler::save_file_interface;
use crate::vecmath::*;
use crate::octree::*;
use crate::builder::*;
use crate::renderer::*;
use crate::state::*;
use crate::cli::*;
use crate::worldgen::generate_random_world;

const DEFAULT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.0];

pub struct Player {
    pub position: V3,
    pub direction: (f32, f32),
}

struct Lighting {
    sun_direction: V3,
    ambient_strength: f32,
    sky_color: [f32; 4],
    time: f32,
    time_scale: f32,
}

struct KeyPresses {
    W: bool,
    A: bool,
    S: bool,
    D: bool,
    Shift: bool,
    Space: bool,
    Ctrl: bool,
    Up: bool,
    Left: bool,
    Down: bool,
    Right: bool
}

impl KeyPresses {
    fn new() -> Self {
        Self {
            W: false,
            A: false,
            S: false,
            D: false,
            Shift: false,
            Space: false,
            Ctrl: false,
            Up: false,
            Left: false,
            Down: false,
            Right: false
        }
    }
}

struct App {
    state: Option<State>,
    chunks: HashMap<V3i, Chunk>,

    worldgen_chunks: HashMap<V3i, Chunk>,
    seed: u32,

    player: Player,
    render_distance: u32,
    max_heap_bytes: u64,
    colours: Vec<[f32; 4]>,
    lighting: Lighting,
    key_presses: KeyPresses,
    last_redraw: Instant,

    last_fps_update: Instant,
    frames_this_second: u32,
    current_acc_fps: f32,
}

enum Direction {
    Forward,
    Back,
    Left,
    Right    
}

impl Player {
    const TWO_PI: f32 = std::f32::consts::PI * 2.0;
    
    fn move_in_direction(&mut self, direction: Direction, step: f32) {
        let quarter_rotation = std::f32::consts::FRAC_PI_2;
        let (dx, dz) = match direction {
            Direction::Forward => (self.direction.0.sin(), self.direction.0.cos()),
            Direction::Back => (-self.direction.0.sin(), -self.direction.0.cos()),
            Direction::Right => ((self.direction.0 + quarter_rotation).sin(), (self.direction.0 + quarter_rotation).cos()),
            Direction::Left=> ((self.direction.0 - quarter_rotation).sin(), (self.direction.0 - quarter_rotation).cos()),
        };

        self.position.x += dx * step;
        self.position.z += dz * step;
    }

    fn move_up(&mut self, step: f32) {
        self.position.y += step;
    }

    fn move_down(&mut self, step: f32) {
        self.position.y -= step;
    }

    fn rotate_yaw(&mut self, angle: f32) { 
        self.direction.0 += angle;

        // Wrap the rotation to prevent it from getting too large
        if self.direction.0 > Self::TWO_PI {
            self.direction.0 -= Self::TWO_PI
        } else if self.direction.0 < Self::TWO_PI {
            self.direction.0 += Self::TWO_PI
        }
    }

    fn rotate_pitch(&mut self, angle: f32) {
        let new = self.direction.1 + angle;

        // Clamp pitch to max out at looking up or down
        self.direction.1 = new.clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
    }
}

impl ApplicationHandler<CliCommand> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                    .with_visible(false)
                    .with_title("Raycaster")
                )
                .unwrap(),
        );

        let packed_world = pack_world_to_gpu(&self.chunks, self.render_distance);

        let state = pollster::block_on(State::new(
            event_loop.owned_display_handle(),
            window.clone(),
            &packed_world,
            self.render_distance,
            self.max_heap_bytes,
        ));
        self.state = Some(state);
        
        window.set_visible(true);
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Generate combined chunks to send to rendering
                let mut new_chunks = HashMap::<V3i, Chunk>::new();
                for key in self.worldgen_chunks.keys() {
                    let val = self.worldgen_chunks.get(key).unwrap();
                    let mut n = Chunk {
                        data: val.data.clone(),
                        min_pos: val.min_pos,
                        max_pos: val.max_pos
                    };

                    if let Some(loaded) = self.chunks.get(&key) {
                        n.add_chunk(loaded);
                    };
                    new_chunks.insert(*key, n);
                };

                // Send the combined chunks to processing
                state.process_chunk_loading(&self.player, self.render_distance, &new_chunks);

                state.render(&self.player, self.render_distance, &self.colours, &self.lighting);
                // Emits a new redraw requested event.
                state.get_window().request_redraw();
                
                //=============================
                //Fps counter:
                self.frames_this_second += 1;

                let elapsed = self.last_fps_update.elapsed();

                if elapsed.as_secs_f32() >= 1.0 {
                    let fps = self.frames_this_second as f32 / elapsed.as_secs_f32();
                    state.window.set_title(&format!("Raycaster - {:.2} FPS", fps));

                    self.frames_this_second = 0;
                    self.last_fps_update = Instant::now();

                    //println!("position");
                    //println!("x: {}, y: {}, z: {}", self.player.position.x, self.player.position.y, self.player.position.z);
                }

                //=============================
                // Worldgen:
                let player_center = self.player.position;
                let radius = self.render_distance as i32;

                // Keep the chunks within the render distance, and remove the other
                self.worldgen_chunks.retain(|key, _| {
                    let dx = (key.x - ((player_center.x / 32.0) as i32)).abs() as u32;
                    let dy = (key.y - ((player_center.y / 32.0) as i32)).abs() as u32;
                    let dz = (key.z - ((player_center.z / 32.0) as i32)).abs() as u32;

                    dx <= self.render_distance && dy <= self.render_distance && dz <= self.render_distance
                });
                
                for y in 0..=5{
                    for x in (((player_center.x / 32.0) as i32) - radius)..=(((player_center.x / 32.0) as i32) + radius) {
                        for z in (((player_center.z / 32.0) as i32) - radius)..=(((player_center.z / 32.0) as i32) + radius) {
                            let pos = V3i {x, y, z};

                            // Skip if exists
                            if let Some(_) = self.worldgen_chunks.get(&pos) {
                                continue;
                            };

                            // Create chunks
                            let colors = worldgen::BlockColors { grass: 1, stone: 1, water: 1 };
                            let chunk_data = worldgen::generate_single_chunk(&colors, self.seed, &pos);
                            let data = build_chunk(&chunk_data);
                            let chunk = Chunk { data, min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 }, max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 }};

                            // Add chunk
                            self.worldgen_chunks.insert(pos, chunk);
                        }
                    }
                }


                //=============================
                //movement
                let delta_time = Instant::now().duration_since(self.last_redraw).as_secs_f32();
                let mut move_speed = 10.0;
                let rot_speed = std::f32::consts::FRAC_PI_2 * 1.5;
                self.last_redraw = Instant::now();
                // Shift
                if self.key_presses.Shift {
                    move_speed *= 10.0;
                }

                // WASD movement
                if self.key_presses.W {
                    self.player.move_in_direction(Direction::Forward, move_speed * delta_time);
                }
                if self.key_presses.A {
                    self.player.move_in_direction(Direction::Left, move_speed * delta_time);
                }
                if self.key_presses.S {
                    self.player.move_in_direction(Direction::Back, move_speed * delta_time);
                }
                if self.key_presses.D {
                    self.player.move_in_direction(Direction::Right, move_speed * delta_time);
                }

                // Up / Down
                if self.key_presses.Space {
                    self.player.move_up(move_speed * delta_time);
                }
                if self.key_presses.Ctrl {
                    self.player.move_down(move_speed * delta_time);
                }


                if self.key_presses.Up {
                    self.player.rotate_pitch(rot_speed * delta_time);
                }
                if self.key_presses.Down {
                    self.player.rotate_pitch(-rot_speed * delta_time);
                }
                if self.key_presses.Left {
                    self.player.rotate_yaw(-rot_speed * delta_time);
                }
                if self.key_presses.Right {
                    self.player.rotate_yaw(rot_speed * delta_time);
                }
                //=============================
                //Time 
                self.lighting.time += self.lighting.time_scale * delta_time;

                if self.lighting.time > Player::TWO_PI {
                    self.lighting.time -= Player::TWO_PI;
                }

                let raw_sun_dir = V3 {
                    x: self.lighting.time.cos(),
                    y: self.lighting.time.sin(),
                    z: 0.3, 
                };

                self.lighting.sun_direction = vec_normalize(&raw_sun_dir);

            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);

                //Fps reset:
                self.last_fps_update = Instant::now();
                self.frames_this_second = 0;
                self.current_acc_fps = 0.0;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Ignore if repeated key press
                if event.repeat {
                    return;
                }

                // Get key code and state from press/release
                let key_code = match event.physical_key {
                    winit::keyboard::PhysicalKey::Code(key_code) => key_code,
                    winit::keyboard::PhysicalKey::Unidentified(_) => return,
                };
                let state = match event.state {
                    winit::event::ElementState::Pressed => true,
                    winit::event::ElementState::Released => false,
                };
                
                match key_code {
                    // WASD
                    winit::keyboard::KeyCode::KeyW => {
                        self.key_presses.W = state;
                    }
                    winit::keyboard::KeyCode::KeyA => {
                        self.key_presses.A = state;
                    }
                    winit::keyboard::KeyCode::KeyS => {
                        self.key_presses.S = state;
                    }
                    winit::keyboard::KeyCode::KeyD => {
                        self.key_presses.D = state;
                    }

                    // Space
                    winit::keyboard::KeyCode::Space => {
                        self.key_presses.Space = state;
                    }
                    
                    // Modifiers
                    winit::keyboard::KeyCode::ShiftLeft => {
                        self.key_presses.Shift = state;
                    }
                    winit::keyboard::KeyCode::ControlLeft => {
                        self.key_presses.Ctrl = state;
                    }

                    // Arrow keys
                    winit::keyboard::KeyCode::ArrowUp => {
                        self.key_presses.Up = state;
                    }
                    winit::keyboard::KeyCode::ArrowLeft => {
                        self.key_presses.Left = state;
                    }
                    winit::keyboard::KeyCode::ArrowDown => {
                        self.key_presses.Down = state;
                    }
                    winit::keyboard::KeyCode::ArrowRight => {
                        self.key_presses.Right = state;
                    }
                    _ => {}
                }
            }
            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, cmd: CliCommand) {
        execute_cli_commands(self, event_loop, cmd);
    }
}

fn main() {
    env_logger::init();

    let event_loop: EventLoop<CliCommand> = EventLoop::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    event_loop.set_control_flow(ControlFlow::Poll);

    std::thread::spawn(move || {
        let stdin = std::io::stdin();

        while let Some(Ok(line)) = stdin.lock().lines().next() {
            match parse_command(&line) {
                Some(cmd) => {
                    if proxy.send_event(cmd).is_err() {
                        break; 
                    }
                }
                None => eprintln!("Unknown command: '{}', use 'help' for list of all avaliable commands", line),
            }
        }
    });

    let chunks = HashMap::new();
    let colours = vec![DEFAULT_COLOR, DEFAULT_COLOR];
    let player = Player {
        position: V3{
            x: 32.0*2.0,
            y: 32.0*2.0,
            z: 32.0*2.0,
        },
        //direction: (0.0, -std::f32::consts::FRAC_PI_2)               
        direction: (std::f32::consts::FRAC_PI_3, 0.0)               
    };

    let lighting = Lighting { 
        sun_direction: V3{x: 0.0, y: 1.0, z: 0.0},
        ambient_strength: 0.2,
        sky_color: [0.5, 0.7, 1.0, 1.0],
        time_scale: 0.01,
        time: 0.8,
    };

    let mut app = App {
        state: None,
        chunks, 
        last_fps_update: Instant::now(),
        frames_this_second: 0,
        player,
        current_acc_fps: 0.0,
        //använd bara 2^a render distances, ex: 4,8,16,32,64 ...
        render_distance: 32,
        max_heap_bytes: 512 * 1024 * 1024, // 512 MB
        colours,
        lighting,
        key_presses: KeyPresses::new(),
        last_redraw: Instant::now(),
        worldgen_chunks: HashMap::new(),
        seed: 1227,

    };

    println!("Launching Raycaster...");
    event_loop.run_app(&mut app).unwrap();
}
