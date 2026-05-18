use std::sync::Arc;
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use winit::{
    event_loop::{OwnedDisplayHandle},
    window::{Window},
};

use crate::renderer::render_starter;
use crate::{Player, Lighting};
use crate::vecmath::*;
use crate::octree::*;


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PlayerUniform {
    pub position: [f32; 3],
    pub render_distance: u32,
    pub top_left: [f32; 3],
    pub _padding2: u32,
    pub delta_x: [f32; 3],
    pub _padding3: u32,
    pub delta_y: [f32; 3],
    pub _padding4: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub sun_direction: [f32; 3],  
    pub ambient_strength: f32,      
    pub face_multipliers_1: [f32; 4], 
    pub face_multipliers_2: [f32; 4], 
    pub sky_color: [f32; 4],
}

pub struct State {
    pub instance: wgpu::Instance,
    pub window: Arc<Window>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,

    pub allocator: VoxelHeapAllocator,
    pub active_chunks: HashMap<V3i, ChunkAllocation>,
    pub grid_size: u32,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub ray_start_pipeline: wgpu::ComputePipeline,
    pub shading_pipeline: wgpu::ComputePipeline,
    
    pub hit_buffer: wgpu::Buffer,
    pub player_buffer: wgpu::Buffer,
    pub world_buffer: wgpu::Buffer,
    pub color_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
}

// Represents an active chunk stored on the GPU
pub struct ChunkAllocation {
    pub start_idx: u32,
    pub length: u32,
}

pub struct VoxelHeapAllocator {
    pub free_blocks: Vec<(u32, u32)>, 
    pub capacity: u32,
}

impl VoxelHeapAllocator {
    pub fn new(start_offset: u32, capacity: u32) -> Self {
        Self {
            free_blocks: vec![(start_offset, capacity - start_offset)],
            capacity,
        }
    }

    pub fn allocate(&mut self, size: u32) -> Option<u32> {
        if let Some(idx) = self.free_blocks.iter().position(|block| block.1 >= size) {
            let block = self.free_blocks[idx];
            let allocated_start = block.0;

            if block.1 == size {
                self.free_blocks.remove(idx);
            } else {
                self.free_blocks[idx] = (block.0 + size, block.1 - size);
            }
            return Some(allocated_start);
        }
        //No more Vram
        None 
    }

    pub fn free(&mut self, start: u32, size: u32) {
        self.free_blocks.push((start, size));
        // Implement a merge function here to combine adjacent free blocks 
        // to prevent fragmentation over time.
    }
}

impl State {
    pub async fn new(display: OwnedDisplayHandle, window: Arc<Window>, gpu_world_data: &[u32], render_distance: u32, desired_heap_bytes: u64) -> State {
        let mut descriptor = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display));
        descriptor.backends = wgpu::Backends::VULKAN;
        descriptor.flags |= wgpu::InstanceFlags::DEBUG;
        let instance = wgpu::Instance::new(descriptor);

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();


        let supported_limits = adapter.limits();
        let absolute_max_buffer = supported_limits.max_storage_buffer_binding_size as u64;

        let target_buffer_bytes = absolute_max_buffer.min(desired_heap_bytes);
        let final_buffer_bytes = target_buffer_bytes & !3;

        let max_u32_elements = (final_buffer_bytes / 4) as u32;
        println!("Allocating a Voxel Heap of {} MB", final_buffer_bytes / 1024 / 1024);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::BGRA8UNORM_STORAGE,
                    required_limits: wgpu::Limits {
                        max_storage_buffer_binding_size: final_buffer_bytes,
                        max_buffer_size: final_buffer_bytes,
                        ..wgpu::Limits::default()
                    },
                    ..Default::default()
                }
            )
            .await
            .unwrap();


        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();

        let surface_format = wgpu::TextureFormat::Bgra8Unorm;

        let initial_player = PlayerUniform {
            position: [0.0, 0.0, 0.0],
            render_distance,
            top_left: [0.0, 0.0, 0.0],
            _padding2: 0,
            delta_x: [0.0, 0.0, 0.0],
            _padding3: 0,
            delta_y: [0.0, 0.0, 0.0],
            _padding4: 0,
        };

        let initial_light = LightUniform {
            sun_direction: [0.0, 0.0, 0.0],
            ambient_strength: 0.0,
            face_multipliers_1: [0.0, 0.0, 0.0, 0.0], 
            face_multipliers_2: [0.0, 0.0, 0.0, 0.0], 
            sky_color: [0.0, 0.0, 0.0, 1.0],
        };

        let grid_size = (render_distance * 2 + 1).next_power_of_two();
        let indexer_size = grid_size * grid_size * grid_size;

        let pixel_count = (size.width * size.height) as wgpu::BufferAddress;

        let hit_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hit Buffer"),
            size: pixel_count * 32,  
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let player_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Player Uniform Buffer"),
            contents: bytemuck::cast_slice(&[initial_player]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Uniform Buffer"),
            contents: bytemuck::cast_slice(&[initial_light]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let world_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("World Storage Buffer (Dynamic)"),
            size: (final_buffer_bytes) as wgpu::BufferAddress, 
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, 
            mapped_at_creation: false,
        });

        let color_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Color LUT Buffer"),
            size: (65536 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress, 
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let allocator = VoxelHeapAllocator::new(indexer_size, max_u32_elements);

        let empty_indexer = vec![0xFFFFFFFFu32; indexer_size as usize];
        queue.write_buffer(&world_buffer, 0, bytemuck::cast_slice(&empty_indexer));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Main Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform, 
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,  
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform, 
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

        let ray_start_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Ray Gen Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("ray_gen_pass"), 
            compilation_options: Default::default(),
            cache: None,
        });

        let shading_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Shading Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("shading_pass"),
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
            hit_buffer,
            ray_start_pipeline,
            shading_pipeline,
            bind_group_layout,
            player_buffer,
            world_buffer,
            color_buffer,
            light_buffer,
            allocator,
            active_chunks: HashMap::new(),
            grid_size,
        };

        state.configure_surface();
        state
    }

    pub fn set_chunk(&mut self, chunk_pos: V3i, data: Option<&[u32]>) {
        let local_x = chunk_pos.x.rem_euclid(self.grid_size as i32) as u32;
        let local_y = chunk_pos.y.rem_euclid(self.grid_size as i32) as u32;
        let local_z = chunk_pos.z.rem_euclid(self.grid_size as i32) as u32;

        let morton_idx = crate::builder::get_morton_index(local_x, local_y, local_z);  
        let indexer_byte_offset = (morton_idx as u64) * 4;

        if let Some(allocation) = self.active_chunks.remove(&chunk_pos) {
            self.allocator.free(allocation.start_idx, allocation.length);
        }

        if let Some(svo_data) = data {
            let ptr = self.allocator.allocate(svo_data.len() as u32)
                .expect("CRITICAL: Out of VRAM");

            let heap_byte_offset = (ptr as u64) * 4;
            self.queue.write_buffer(&self.world_buffer, heap_byte_offset, bytemuck::cast_slice(svo_data));

            self.queue.write_buffer(&self.world_buffer, indexer_byte_offset, bytemuck::cast_slice(&[ptr]));

            self.active_chunks.insert(chunk_pos, ChunkAllocation {
                start_idx: ptr,
                length: svo_data.len() as u32,
            });
        } else {
            self.queue.write_buffer(&self.world_buffer, indexer_byte_offset, bytemuck::cast_slice(&[0xFFFFFFFFu32]));
        }
    }

    pub fn process_chunk_loading(&mut self, player_pos: &crate::Player, render_distance: u32, all_world_chunks: &HashMap<V3i, Chunk>) {
        let chunk_size = 32.0;
        let center_cx = (player_pos.position.x / chunk_size).floor() as i32;
        let center_cy = (player_pos.position.y / chunk_size).floor() as i32;
        let center_cz = (player_pos.position.z / chunk_size).floor() as i32;

        let radius = render_distance as i32;

        // 1. Unload chunks that are now too far away
        let mut to_remove = Vec::new();
        for pos in self.active_chunks.keys() {
            if (pos.x - center_cx).abs() > radius || 
               (pos.y - center_cy).abs() > radius || 
               (pos.z - center_cz).abs() > radius {
                to_remove.push(*pos);
            }
        }

        for pos in to_remove {
            self.set_chunk(pos, None);         }

        for x in -radius..=radius {
            for y in -radius..=radius {
                for z in -radius..=radius {
                    let target_pos = V3i { x: center_cx + x, y: center_cy + y, z: center_cz + z };
                    
                    if !self.active_chunks.contains_key(&target_pos) {
                        if let Some(chunk) = all_world_chunks.get(&target_pos) {
                            self.set_chunk(target_pos, Some(&chunk.data));
                        }
                    }
                }
            }
        }
    }

    pub fn get_window(&self) -> &Window {
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

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.configure_surface();

            // 1. Calculate the new total number of pixels
            let pixel_count = (new_size.width * new_size.height) as wgpu::BufferAddress;

            self.hit_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Hit Buffer (Resized)"),
                size: pixel_count * 32,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
    }

    pub fn render(&mut self, player: &Player, render_distance: u32, colours: &[[f32; 4]], lighting: &Lighting) {
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

        let fov = std::f32::consts::PI / 2.0;
        let result: (V3, V3, V3) = render_starter(self.size.width, self.size.height, fov, player.direction);

        let player_uniform = PlayerUniform {
            position: [player.position.x, player.position.y, player.position.z],
            render_distance,
            top_left: [result.0.x, result.0.y, result.0.z],
            _padding2: 0,
            delta_x: [result.1.x, result.1.y, result.1.z],
            _padding3: 0,
            delta_y: [result.2.x, result.2.y, result.2.z],
            _padding4: 0,
        };

        let sun = lighting.sun_direction;
        let sun_len = (sun.x * sun.x + sun.y * sun.y + sun.z * sun.z).sqrt();
        let (dir_x, dir_y, dir_z) = if sun_len > 0.0 {
            (sun.x / sun_len, sun.y / sun_len, sun.z / sun_len)
        } else {
            (0.0, 1.0, 0.0) // Fallback to avoid division by zero
        };

        let calc_lighting = |dot: f32| -> f32 {
            lighting.ambient_strength + (1.0 - lighting.ambient_strength) * dot.max(0.0)
        };

        let light_uniform = LightUniform {
            sun_direction: [lighting.sun_direction.x, lighting.sun_direction.y, lighting.sun_direction.z], // Direction TO the sun for shadow rays
            ambient_strength: lighting.ambient_strength,
            
            face_multipliers_1: [calc_lighting(dir_x), calc_lighting(-dir_x), calc_lighting(dir_y), calc_lighting(-dir_y)], 
            face_multipliers_2: [calc_lighting(dir_z), calc_lighting(-dir_z), 0.0, 0.0], 
            sky_color: lighting.sky_color,
        };
        
        self.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[light_uniform]));
        self.queue.write_buffer(&self.player_buffer, 0, bytemuck::cast_slice(&[player_uniform]));
        self.queue.write_buffer(&self.color_buffer, 0, bytemuck::cast_slice(colours));

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
                    binding: 2, // SVO Chunks  
                    resource: self.world_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3, // Färgdatabasen
                    resource: self.color_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4, // Träffdata 
                    resource: self.hit_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding:5, // Ljus
                    resource: self.light_buffer.as_entire_binding(),
                }
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // STARTA COMPUTE-PASSET
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Beräkna 8x8 grupper över skärmens yta
            let workgroups_x = (self.size.width + 7) / 8;
            let workgroups_y = (self.size.height + 7) / 8;
            
            // 1
            compute_pass.set_pipeline(&self.ray_start_pipeline);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);

            // 2
            compute_pass.set_pipeline(&self.shading_pipeline);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        } 

        // SUBMIT OCH PRESENT
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();     
    }
}

