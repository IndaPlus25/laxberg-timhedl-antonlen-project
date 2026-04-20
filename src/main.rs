mod file_parser;
mod voxelizer;

mod octree;
mod vecmath;
mod renderer;
mod builder;
mod worldgen;

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use std::time::Instant; 

use crate::vecmath::*;
use crate::renderer::*;
use crate::octree::*;
use crate::builder::*;

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    chunks: HashMap<V3i, Chunk>,

    last_fps_update: Instant,
    frames_this_second: u32,
    player: Player,
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    // println!("Generating random world data...");
    // let world_data = worldgen::generate_random_world(256, 256, 256, 0.5, 4);

    let mesh = file_parser::file_parse_interface("Susan.obj").unwrap()[0].clone();
    let world_data = voxelizer::voxel_grid_from_triangles(mesh, 50);

    println!("Compressing world into Sparse Voxel Octrees...");
    let chunks = to_chunks(&world_data);
    println!("Successfully built {} chunks!", chunks.len());

    let player = Player {
        position: V3{
            x: -60.5,
            y: 20.1,
            z: 0.1,
        },
        // direction: (0.0, -std::f32::consts::FRAC_PI_2)               
        direction: (std::f32::consts::FRAC_PI_3, 0.0)               
    };

    let mut app = App {
        window: None,
        surface: None,
        chunks,

        last_fps_update: Instant::now(),
        frames_this_second: 0,

        player,
    };

    println!("Launching Raycaster...");
    let _ = event_loop.run_app(&mut app);
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Raycaster");

            let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

            let context = Context::new(window.clone()).unwrap();
            let surface = Surface::new(&context, window.clone()).unwrap();

            self.window = Some(window);
            self.surface = Some(surface);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface && size.width > 0 && size.height > 0 {
                    surface.resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    ).unwrap();
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(surface), Some(window)) = (&mut self.surface, &self.window) {
                    let mut buffer = surface.buffer_mut().unwrap();
                    
                    let size = window.inner_size();
                    let width = size.width;
                    let height = size.height;

                    let fov = std::f32::consts::PI / 2.0;


                    raycaster(&mut buffer, width, height, fov, &self.player, &self.chunks);

                    self.player.direction.0 += 0.01;
                    self.player.position.x += 0.01;
                    
                    buffer.present().unwrap();

                    //Fps counter:
                    self.frames_this_second += 1;

                    let elapsed = self.last_fps_update.elapsed();

                    if elapsed.as_secs_f32() >= 1.0 {
                        let fps = self.frames_this_second as f32 / elapsed.as_secs_f32();
                        window.set_title(&format!("Raycaster - {:.2} FPS", fps));

                        self.frames_this_second = 0;
                        self.last_fps_update = Instant::now();
                    }
                }
                
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
