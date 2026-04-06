mod octree;
mod vecmath;
mod renderer;

use std::num::NonZeroU32;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::vecmath::*;
use crate::renderer::*;
use crate::octree::*;

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        surface: None,
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
                            x: -32.0,
                            y: 45.0,
                            z: 45.0,
                        },
                        direction: (std::f32::consts::PI / 1.5, 0.0)               
                    };

                    let mut tree_data = vec![0_u32; 40];

                    tree_data[0] = (73 << 24) | (72 << 16) | 1; 
                    tree_data[1] = (128 << 24) | (128 << 16) | 4; 
                    tree_data[2] = 2; 
                    tree_data[3] = 3; 
                    tree_data[4] = 1;

                    let chunk = Chunk {
                        data: tree_data,
                        min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
                        max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 },
                    };

                    raycaster(&mut buffer, width, height, fov, player, &chunk);
                    
                    buffer.present().unwrap();
                }
                
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
