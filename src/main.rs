use std::num::NonZeroU32;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Window, WindowId};
use vecmath

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
}

//32x32x32 chunk
struct Chunk {
    //first 8 bits are bools for children existing in each of the 8 positions. Z-order curve
    //sencond u bits are bools for if children are leaf nodes or are parents themselves.
    //last 16 bits are primarily pointers to the first child of current node. If they are a leaf
    //then they save the u8(u16) bit information about its material.
    ///0xCC(child)LL(leaf)OOOO(first_child_pointer)
    data: Vec![u32],
    ///bottom, left, near corner position
    position: Vector3,
}

fn find_intersection(collision_pos: Vector3, direction_vec: Vector3, chunk: Chunk) {

    //Use Parametric Octree Traversal

    let local_collision = vec3_sub(collision_pos, chunk.position) //in chunk collision_pos
    let inner_size = 16;

    


}

fn vec_min(v1: V3, v2: V3) {
    
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
                if let Some(surface) = &mut self.surface {
                    if size.width > 0 && size.height > 0 {
                        surface.resize(
                            NonZeroU32::new(size.width).unwrap(),
                            NonZeroU32::new(size.height).unwrap(),
                        ).unwrap();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(surface), Some(window)) = (&mut self.surface, &self.window) {
                    let mut buffer = surface.buffer_mut().unwrap();
                    
                    let size = window.inner_size();
                    let width = size.width;
                    let height = size.height;

                    default_color(&mut buffer, width, height);
                    
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

fn default_color(buffer: &mut [u32], width: u32, height: u32) {
    for (index, pixel) in buffer.iter_mut().enumerate() {
        let x = (index % width as usize) as f32;
        let y = (index / width as usize) as f32;

        let r = ((x / width as f32) * 255.0) as u32;
        let g = ((y / height as f32) * 255.0) as u32;
        let b = (((x + y) / (width + height) as f32) * 255.0) as u32;

        // Format: 0000_RRRR_GGGG_BBBB
        *pixel = (r << 16) | (g << 8) | b;
    }
}



