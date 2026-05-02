mod file_parser;
mod voxelizer;

mod error;
mod file_handler;

mod octree;
mod vecmath;
mod builder;
mod worldgen;

use std::collections::HashMap;
use std::rc::Rc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowId},
};
use std::time::Instant; 
use std::sync::Arc;
use wgpu::util::DeviceExt;

use crate::file_handler::save_file_interface;
use crate::vecmath::*;
use crate::octree::*;
use crate::builder::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PlayerUniform {
    pub position: [f32; 3],
    pub _padding: u32,
    pub direction: [f32; 2],
    pub fov: f32,
    pub aspect_ratio: f32,
}

pub struct Player {
    pub position: V3,
    pub direction: (f32, f32),
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

    player: Player,
    key_presses: KeyPresses,
    last_redraw: Instant,

    last_fps_update: Instant,
    frames_this_second: u32,
}

struct State {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,

    compute_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,

    player_buffer: wgpu::Buffer,
    world_buffer: wgpu::Buffer,
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

impl State {
    async fn new(display: OwnedDisplayHandle, window: Arc<Window>, gpu_world_data: &[u32]) -> State {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::BGRA8UNORM_STORAGE,
                    ..Default::default()
                }
            )
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();

        let surface_format = wgpu::TextureFormat::Bgra8Unorm;

        // 1. SKAPA PLAYER BUFFER (Uniform)
        let initial_player = PlayerUniform {
            position: [0.0, 0.0, 0.0],
            _padding: 0,
            direction: [0.0, 0.0],
            fov: 90.0_f32.to_radians(),
            aspect_ratio: size.width as f32 / size.height as f32,
        };

        let player_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Player Uniform Buffer"),
            contents: bytemuck::cast_slice(&[initial_player]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 2. SKAPA WORLD BUFFER (Storage)
        let world_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("World Storage Buffer"),
            contents: bytemuck::cast_slice(gpu_world_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 3. SKAPA BIND GROUP LAYOUT (Bron)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Main Bind Group Layout"),
            entries: &[
                // Binding 0: Kamera (Uniform Buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        // VIKTIGT: Nu är det en Uniform!
                        ty: wgpu::BufferBindingType::Uniform, 
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: Skärmen (Storage Texture)
            wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: surface_format,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Binding 2: Världsdatan / Chunks (Storage Buffer, Read Only)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        // VIKTIGT: SVO-datan är gigantisk, så det måste vara Storage
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // 4. LADDA SHADERN OCH SKAPA PIPELINE
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],

            immediate_size: 0,
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let state = State {
            instance,
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            compute_pipeline,
            bind_group_layout,
            player_buffer,
            world_buffer,
        };

        state.configure_surface();
        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::STORAGE_BINDING,
            format: self.surface_format,
            view_formats: vec![],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn render(&mut self, player: &Player) {
        // Create texture view.
        // NOTE: We must handle Timeout because the surface may be unavailable
        // (e.g., when the window is occluded on macOS).
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.configure_surface();
                return;
            }
        };

        let texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let aspect_ratio = self.size.width as f32 / self.size.height as f32;
        let player_uniform = PlayerUniform {
            position: [player.position.x, player.position.y, player.position.z],
            _padding: 0,
            direction: [player.direction.0, player.direction.1], // yaw och pitch
            fov: 90.0_f32.to_radians(), // Du kan göra FOV dynamisk senare!
            aspect_ratio,
        };
        // Skriv över datan i VRAM
        self.queue.write_buffer(&self.player_buffer, 0, bytemuck::cast_slice(&[player_uniform]));

        // --- BYGG BRON (Med alla 3 bindings) ---
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, // Uniform Kameran
                    resource: self.player_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1, // Skärmen
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2, // SVO Chunks (Just nu vår dummy_world_data)
                    resource: self.world_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // STARTA COMPUTE-PASSET
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Beräkna 8x8 grupper över skärmens yta
            let workgroups_x = (self.size.width + 7) / 8;
            let workgroups_y = (self.size.height + 7) / 8;
            
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        } // Compute pass droppas här

        // SUBMIT OCH PRESENT
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present(); // Fixad!
    }
}


impl ApplicationHandler for App {
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

        let packed_world = pack_world_to_gpu(&self.chunks);

        let state = pollster::block_on(State::new(
            event_loop.owned_display_handle(),
            window.clone(),
            &packed_world,
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
                state.render(&self.player);
                // Emits a new redraw requested event.
                state.get_window().request_redraw();

                //Fps counter:
                self.frames_this_second += 1;

                let elapsed = self.last_fps_update.elapsed();

                if elapsed.as_secs_f32() >= 1.0 {
                    let fps = self.frames_this_second as f32 / elapsed.as_secs_f32();
                    state.window.set_title(&format!("Raycaster - {:.2} FPS", fps));

                    self.frames_this_second = 0;
                    self.last_fps_update = Instant::now();
                }

                let delta_time = Instant::now().duration_since(self.last_redraw).as_secs_f32();
                let move_speed = 10.0;
                let rot_speed = std::f32::consts::FRAC_PI_2 * 1.5;
                self.last_redraw = Instant::now();

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
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
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
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mesh = file_parser::file_parse_interface("Susan.obj").unwrap().clone();
    let world_data = voxelizer::voxel_grid_from_triangles(mesh, 100);

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
        state: None,
        chunks, 
        last_fps_update: Instant::now(),
        frames_this_second: 0,
        player,
        key_presses: KeyPresses::new(),
        last_redraw: Instant::now(),
    };

    println!("Launching Raycaster...");
    event_loop.run_app(&mut app).unwrap();
}
