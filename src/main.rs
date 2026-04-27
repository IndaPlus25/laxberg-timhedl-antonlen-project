mod file_parser;
mod voxelizer;
mod error;

mod octree;
mod vecmath;
mod renderer;
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

use crate::vecmath::*;
use crate::renderer::*;
use crate::octree::*;
use crate::builder::*;

struct App {
    state: Option<State>,
    chunks: HashMap<V3i, Chunk>,

    last_fps_update: Instant,
    frames_this_second: u32,
    player: Player,

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
    color_buffer: wgpu::Buffer,
}

impl State {
    async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> State {
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
                    // Vi ber fortfarande om tillåtelse att skriva till skärmen!
                    required_features: wgpu::Features::BGRA8UNORM_STORAGE,
                    ..Default::default()
                }
            )
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);

        let surface_format = wgpu::TextureFormat::Bgra8Unorm;

        // 1. SKAPA FÄRGDATAN (Lila)
        let my_cpu_color: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

        // 2. LADDA UPP FÄRGEN TILL GPU:n
        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Color Buffer"),
            contents: bytemuck::cast_slice(&my_cpu_color),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 3. SKAPA BIND GROUP LAYOUT (Bron)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Main Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // Vår color_buffer
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, // Vår skärm
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: surface_format,
                        view_dimension: wgpu::TextureViewDimension::D2,
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
            color_buffer,
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
            present_mode: wgpu::PresentMode::AutoNoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn render(&mut self) {
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

        // BYGG BRON FÖR DENNA BILDRUTA
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.color_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
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

        let state = pollster::block_on(State::new(
            event_loop.owned_display_handle(),
            window.clone(),
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
                state.render();
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
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
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
        state: None,
        chunks, 
        last_fps_update: Instant::now(),
        frames_this_second: 0,
        player, 
    };

    println!("Launching Raycaster...");
    event_loop.run_app(&mut app).unwrap();
}
