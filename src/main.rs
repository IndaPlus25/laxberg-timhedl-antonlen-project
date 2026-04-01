use std::num::NonZeroU32;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
}

#[derive(Copy, Clone)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Copy, Clone)]
struct Ray {
    origin: V3,
    direction: V3 //should be normalized
}

struct IntersectionData {
    ray: Ray,
    voxel_data: u32,
}

//32x32x32 chunk
struct Chunk {
    //first 8 bits are bools for children(1) existing in each of the 8 positions. Z-order curve
    //sencond 8 bits are bools for if children are leaf nodes(1) or are parents themselves(0).
    //last 16 bits are primarily pointers to the first child of current node. If they are a leaf
    //then they save the u8(u16) bit information about its material.
    ///0xCC(child)LL(leaf)OOOO(first_child_pointer)
    data: Vec<u32>,
    ///bottom, left, near corner position minimum position
    min_pos: V3,
    ///top, right, far corner position maximum position
    max_pos: V3,
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

///None = no intersectin in chunk by ray, else IntersectionData
fn find_intersection(ray: &Ray, chunk: &Chunk, current: u32) -> Option<IntersectionData> {

    let mut direction_mask: u32 = 0;
    let mut pos_ray_dir: V3 = ray.direction;
    let mut pos_ray_origin: V3 = ray.origin;

    if pos_ray_dir.x < 0.0 {
        direction_mask |= 1;
        pos_ray_dir.x = -pos_ray_dir.x;
        pos_ray_origin.x = chunk.max_pos.x - (ray.origin.x - chunk.min_pos.x);
    }
    if pos_ray_dir.y < 0.0 {
        direction_mask |= 2;
        pos_ray_dir.y = -pos_ray_dir.y;
        pos_ray_origin.y = chunk.max_pos.y - (ray.origin.y - chunk.min_pos.y);
    }
    if pos_ray_dir.z < 0.0 {
        direction_mask |= 4;
        pos_ray_dir.z = -pos_ray_dir.z;
        pos_ray_origin.z = chunk.max_pos.z - (ray.origin.z - chunk.min_pos.z);
    }

    let entry = vec_div(&vec_sub(&chunk.min_pos, &pos_ray_origin), &pos_ray_dir);
    let exit = vec_div(&vec_sub(&chunk.max_pos, &pos_ray_origin), &pos_ray_dir);

    let t_min = entry.x.max(entry.y).max(entry.z);
    let t_max = exit.x.min(exit.y).min(exit.z);

    if t_min >= t_max {
        return None; 
    }
    if t_max < 0.0 {
        return None; 
    }

    proc_subtree(ray, chunk, current, entry, exit, direction_mask)

}

fn proc_subtree(ray: &Ray, chunk: &Chunk, current: u32, entry: V3, exit: V3, direction_mask: u32) -> Option<IntersectionData>{
    
    let mid = vec_mult_scal(&vec_add(&entry, &exit), 0.5);

    let entry_plane = vec_entry_plane(&entry);

    //000 is child 0 111 is child 7
    let mut first_child_intersect: u32 = 0; 

    if entry_plane == 0 {
        if mid.y < entry.x {
            first_child_intersect |= 2;
        }
        if mid.z < entry.x {
            first_child_intersect |= 4;
        }
    } else if entry_plane == 1 {
        if mid.x < entry.y {
            first_child_intersect |= 1;
        }
        if mid.z < entry.y {
            first_child_intersect |= 4;
        }
    } else {
        if mid.x < entry.z {
            first_child_intersect |= 1;
        }
        if mid.y < entry.z {
            first_child_intersect |= 2;
        }
    }

    let mut current_sub_voxel: u32 = first_child_intersect;

    loop {


        let true_sub_voxel: u32 = current_sub_voxel ^ direction_mask;

        if has_child(current, true_sub_voxel) {

            let voxel_data = get_ending(current);

            if is_leaf(current, true_sub_voxel) {
                return Some(IntersectionData { ray: *ray, voxel_data, });
            } else {
                let next = chunk.data[voxel_data as usize];

                let sub_entry = V3 {
                    x: if (current_sub_voxel & 1) != 0 { mid.x } else { entry.x },
                    y: if (current_sub_voxel & 2) != 0 { mid.y } else { entry.y },
                    z: if (current_sub_voxel & 4) != 0 { mid.z } else { entry.z },
                };

                let sub_exit = V3 {
                    x: if (current_sub_voxel & 1) != 0 { exit.x } else { mid.x },
                    y: if (current_sub_voxel & 2) != 0 { exit.y } else { mid.y },
                    z: if (current_sub_voxel & 4) != 0 { exit.z } else { mid.z },
                };

                let result = proc_subtree(ray, chunk, next, sub_entry, sub_exit, direction_mask);
                
                if result.is_some() {
                    return result;
                }
            }
        }

        let node_exit: V3 = V3 {
            x: if (current_sub_voxel & 1) != 0 { exit.x } else { mid.x },
            y: if (current_sub_voxel & 2) != 0 { exit.y } else { mid.y },
            z: if (current_sub_voxel & 4) != 0 { exit.z } else { mid.z },
        };
        let exit_plane = vec_exit_plane(&node_exit);

        current_sub_voxel = match (current_sub_voxel, exit_plane) {
            (0, 0) => 1, (0, 1) => 2, (0, 2) => 4,
            (1, 0) => return None, (1, 1) => 3, (1, 2) => 5,
            (2, 0) => 3, (2, 1) => return None, (2, 2) => 6,
            (3, 0) => return None, (3, 1) => return None, (3, 2) => 7,
            (4, 0) => 5, (4, 1) => 6, (4, 2) => return None,
            (5, 0) => return None, (5, 1) => 7, (5, 2) => return None,
            (6, 0) => 7, (6, 1) => return None, (6, 2) => return None,
            (7, _) => return None, 
            _ => return None, 
        };    
    }
}

fn get_ending(data: u32) -> u32 {
    data & 0xFFFF
}

fn is_leaf(data: u32, position: u32) -> bool {
    let n = 1_u32 << (position + 16);

    (data & n) != 0
}

fn has_child(data: u32, position: u32) -> bool {
    let n = 1_u32 << (position + 24);

    (data & n) != 0
}

fn vec_add(v1: &V3, v2: &V3) -> V3 {
    V3 {
        x: v1.x + v2.x,
        y: v1.y + v2.y,
        z: v1.z + v2.z,
    }
}

fn vec_sub(v1: &V3, v2: &V3) -> V3 {
    V3 {
        x: v1.x - v2.x,
        y: v1.y - v2.y,
        z: v1.z - v2.z,
    }
}

fn vec_div(v1: &V3, v2: &V3) -> V3 {
    V3 {
        x: v1.x / v2.x,
        y: v1.y / v2.y,
        z: v1.z / v2.z,
    }
}

fn vec_mult_scal(v1: &V3, n: f32) -> V3 {
    V3{
        x: v1.x * n,
        y: v1.y * n,
        z: v1.z * n,
    }
}

//Buildning the 3 bit intersection identifier (000 - 0(left/right)0(bottom/top)0(fron/back))
fn vec_entry_plane(v1: &V3) -> u32 {
    if v1.x > v1.y && v1.x > v1.z {
        0 //YZ plane
    } else if v1.y > v1.x && v1.y > v1.z {
        1 //XZ plane
    } else {
        2 //XY plane
    }
}

fn vec_exit_plane(v1: &V3) -> u32 {
    if v1.x < v1.y && v1.x < v1.z {
        0 //YZ
    } else if v1.y < v1.x && v1.y < v1.z {
        1 //XZ
    } else {
        2 //XY
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_math() {
        let v1 = V3 { x: 10.0, y: 20.0, z: 30.0 };
        let v2 = V3 { x: 2.0, y: 4.0, z: 5.0 };

        let added = vec_add(&v1, &v2);
        assert_eq!((added.x, added.y, added.z), (12.0, 24.0, 35.0));

        let subbed = vec_sub(&v1, &v2);
        assert_eq!((subbed.x, subbed.y, subbed.z), (8.0, 16.0, 25.0));

        let divided = vec_div(&v1, &v2);
        assert_eq!((divided.x, divided.y, divided.z), (5.0, 5.0, 6.0));

        let scaled = vec_mult_scal(&v1, 0.5);
        assert_eq!((scaled.x, scaled.y, scaled.z), (5.0, 10.0, 15.0));
    }

    #[test]
    fn test_entry_exit_planes() {
        let entry_x_max = V3 { x: 10.0, y: 5.0, z: 2.0 };
        assert_eq!(vec_entry_plane(&entry_x_max), 0); // YZ plane

        let exit_z_min = V3 { x: 20.0, y: 15.0, z: 5.0 };
        assert_eq!(vec_exit_plane(&exit_z_min), 2); // XY plane
    }

    #[test]
    fn test_bitwise_packing() {
        let mut test_state: u32 = 0;
        
        test_state |= 1 << 24;      // Child 0 exists
        test_state |= 1 << 31;      // Child 7 exists
        
        test_state |= 1 << 16;      // Child 0 is a leaf
        
        let payload: u32 = 0xABCD;
        test_state |= payload;      // Add the payload to the bottom 16 bits
        
        assert!(has_child(test_state, 0), "Failed to find Child 0 in CC byte");
        assert!(has_child(test_state, 7), "Failed to find Child 7 in CC byte");
        assert!(!has_child(test_state, 1), "Falsely found Child 1 in CC byte");
        
        assert!(is_leaf(test_state, 0), "Failed to identify Child 0 as a leaf in LL byte");
        assert!(!is_leaf(test_state, 7), "Falsely identified Child 7 as a leaf in LL byte"); 
        
        assert_eq!(get_ending(test_state), 0xABCD, "Failed to extract OOOO payload");
    }

    #[test]
    fn test_ray_misses_chunk_completely() {
        let chunk = Chunk {
            data: vec![],
            min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
            max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 },
        };

        let ray = Ray {
            origin: V3 { x: 50.0, y: 50.0, z: 50.0 }, // Outside the chunk
            direction: V3 { x: 1.0, y: 1.0, z: 1.0 }, // Pointing AWAY from the chunk
        };

        let result = find_intersection(&ray, &chunk, 0);
        assert!(result.is_none(), "Ray should have missed the chunk completely");
    }

    #[test]
    fn test_direct_hit_on_voxel_zero() {
        //Child 0 exists, Child 0 is leaf, Payload is 0x9999
        let mut root_node_data: u32 = 0;
        root_node_data |= 1 << 24;    // Child 0 exists
        root_node_data |= 1 << 16;    // Child 0 is leaf
        root_node_data |= 0x9999;     // Payload

        let chunk = Chunk {
            data: vec![root_node_data],
            min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
            max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 },
        };

        // Ray starts slightly outside the chunk on the X axis, pointing straight through Voxel 0
        let ray = Ray {
            origin: V3 { x: -5.0, y: 8.0, z: 8.0 }, 
            direction: V3 { x: 1.0, y: 0.0, z: 0.0 }, // Straight right
        };

        let result = find_intersection(&ray, &chunk, root_node_data);
        
        assert!(result.is_some(), "Ray should have hit voxel 0");
        if let Some(intersect) = result {
            // Check if it returned the correct payload (the lower 16 bits of our mock data)
            assert_eq!(intersect.voxel_data, 0x9999, "Returned incorrect payload"); 
        }
    }

    #[test]
    fn test_negative_ray_reflection() {
        // Build the root node data: Child 7 exists, Child 7 is leaf, Payload is 0x7777
        let mut root_node_data: u32 = 0;
        root_node_data |= 1_u32 << 31; // Child 7 exists 
        root_node_data |= 1 << 23;     // Child 7 is leaf
        root_node_data |= 0x7777;      // Payload

        let chunk = Chunk {
            data: vec![root_node_data],
            min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
            max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 },
        };

        // Ray starts outside the top-right-back corner, pointing straight backwards towards the origin
        let ray = Ray {
            origin: V3 { x: 40.0, y: 40.0, z: 40.0 }, 
            // Negative directions! This will trigger your XOR mask logic.
            direction: V3 { x: -0.577, y: -0.577, z: -0.577 }, 
        };

        let result = find_intersection(&ray, &chunk, root_node_data);
        
        // If the XOR mask logic fails, it will think the ray hit Voxel 0 and return None.
        // If the XOR mask works, it will correctly translate the hit to Voxel 7.
        assert!(result.is_some(), "Negative ray reflection failed to hit voxel 7");
        if let Some(intersect) = result {
            assert_eq!(intersect.voxel_data, 0x7777, "Hit the right voxel, but got the wrong data");
        }
    }
}
