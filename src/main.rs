mod octree;
mod vecmath;
mod renderer;

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

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    chunks: HashMap<V3i, Chunk>,

    last_fps_update: Instant,
    frames_this_second: u32,
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut tree_data = vec![0_u32; 40];

    tree_data[0] = (73 << 24) | (72 << 16) | 1; 
    tree_data[1] = (128 << 24) | (128 << 16) | 4; 
    tree_data[2] = 2; 
    tree_data[3] = 3; 
    tree_data[4] = 1;

    let mut chunks = HashMap::new();

    for cx in -3..=3 {
        for cz in -3..=3 {
            let chunk_pos = V3i { x: cx, y: 0, z: cz };
            let chunk = Chunk {
                data: tree_data.clone(),
                min_pos: V3 { x: cx as f32 * 32.0, y: 0.0, z: cz as f32 * 32.0 },
                max_pos: V3 { x: (cx + 1) as f32 * 32.0, y: 32.0, z: (cz + 1) as f32 * 32.0 },
            };
            chunks.insert(chunk_pos, chunk);
        }
    }

    let mut app = App {
        window: None,
        surface: None,
        chunks,

        last_fps_update: Instant::now(),
        frames_this_second: 0,
    };

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

                    let player = Player {
                        position: V3{
                            x: -16.0,
                            y: 45.0,
                            z: -16.0,
                        },
                        direction: (std::f32::consts::PI / 1.5, -0.4)               
                    };

                    raycaster(&mut buffer, width, height, fov, player, &self.chunks);
                    
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
