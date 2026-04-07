use crate::vecmath::*;

use std::collections::HashMap;

//32x32x32 octree optimized chunk
pub struct Chunk {
    //first 8 bits are bools for children(1) existing in each of the 8 positions. Z-order curve
    //sencond 8 bits are bools for if children are leaf nodes(1) or are parents themselves(0).
    //last 16 bits are primarily pointers to the first child of current node. If they are a leaf
    //then they save the u8(u16) bit information about its material.
    ///0xCC(child)LL(leaf)OOOO(first_child_pointer)
    pub data: Vec<u32>,
    ///bottom, left, near corner position minimum position
    pub min_pos: V3,
    ///top, right, far corner position maximum position
    pub max_pos: V3,
}

//32x32x32 non-octree optimized chunk, raw data
pub struct FlatChunk {

    pub data: Vec<u32>,
    pub min_pos: V3,
    pub max_pos: V3,

}

pub fn to_chunks(data: &[&[&[u32]]]) -> Vec<u32> {
    let width = data.len();
    let height = if width > 0 { data[0].len() } else { 0 };
    let depth = if height > 0 { data[0][0].len() } else { 0 };

    let total_size = width * height * depth;

    let mut flat_data = vec![0; total_size];

    for x in 0..width {
        for y in 0..height {
            for z in 0..depth {
                let block = data[x][y][z];
                
                if block != 0 {
                    let index = (x * height * depth) + (y * depth) + z;
                    
                    flat_data[index] = block;
                }
            }
        }
    }

    flat_data
}

pub fn cast_ray(ray: &Ray, chunks: &HashMap<V3i, Chunk>, limit: u32) -> Option<IntersectionData> {
    let chunk_size = 32.0;

    // 1. Initial chunk coordinates (Cleaned up via vecmath helpers)
    let origin_scaled = vec_div_scal(&ray.origin, chunk_size);
    let mut chunk_pos = vec_floor_to_v3i(&origin_scaled);

    // Pre-calculate inverse directions as a V3
    let inv_dir = vec_inv_dir_dda(&ray.direction);

    // 2. Initialization Phase
    let mut step = V3i { x: 0, y: 0, z: 0 };
    let mut t_max = V3 { x: 0.0, y: 0.0, z: 0.0 };
    let mut t_delta = V3 { x: 0.0, y: 0.0, z: 0.0 };

    // X-Axis Setup
    t_delta.x = (chunk_size * inv_dir.x).abs();
    if ray.direction.x > 0.0 {
        step.x = 1;
        t_max.x = (((chunk_pos.x + 1) as f32 * chunk_size) - ray.origin.x) * inv_dir.x;
    } else {
        step.x = -1;
        t_max.x = (ray.origin.x - (chunk_pos.x as f32 * chunk_size)) * -inv_dir.x;
    }

    // Y-Axis Setup
    t_delta.y = (chunk_size * inv_dir.y).abs();
    if ray.direction.y > 0.0 {
        step.y = 1;
        t_max.y = (((chunk_pos.y + 1) as f32 * chunk_size) - ray.origin.y) * inv_dir.y;
    } else {
        step.y = -1;
        t_max.y = (ray.origin.y - (chunk_pos.y as f32 * chunk_size)) * -inv_dir.y;
    }

    // Z-Axis Setup
    t_delta.z = (chunk_size * inv_dir.z).abs();
    if ray.direction.z > 0.0 {
        step.z = 1;
        t_max.z = (((chunk_pos.z + 1) as f32 * chunk_size) - ray.origin.z) * inv_dir.z;
    } else {
        step.z = -1;
        t_max.z = (ray.origin.z - (chunk_pos.z as f32 * chunk_size)) * -inv_dir.z;
    }

    // 3. The Incremental Traversal Loop
    // This perfectly matches Amanatides & Woo: exactly 2 float comparisons, 
    // 1 float add, 2 integer comparisons, and 1 integer add per loop iteration. [cite: 609]
    for _ in 0..limit {
        
        // We now use chunk_pos directly to query the HashMap
        if let Some(chunk) = chunks.get(&chunk_pos) {
            if !chunk.data.is_empty() {
                let root_data = chunk.data[0]; 
                
                if let Some(hit) = find_intersection(ray, chunk, root_data) {
                    return Some(hit);
                }
            }
        }

        // Advance DDA exactly as outlined in the paper's 3D extension [cite: 572-608]
        if t_max.x < t_max.y {
            if t_max.x < t_max.z {
                chunk_pos.x += step.x;
                t_max.x += t_delta.x;
            } else {
                chunk_pos.z += step.z;
                t_max.z += t_delta.z;
            }
        } else {
            if t_max.y < t_max.z {
                chunk_pos.y += step.y;
                t_max.y += t_delta.y;
            } else {
                chunk_pos.z += step.z;
                t_max.z += t_delta.z;
            }
        }
    }

    None 
}
pub fn find_intersection(ray: &Ray, chunk: &Chunk, current: u32) -> Option<IntersectionData> {

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

            let pointer = get_ending(current);

            let child_index = pointer as usize +  child_pop_count(current, true_sub_voxel) as usize;

            let child_node = chunk.data[child_index];

            if is_leaf(current, true_sub_voxel) {

                let material = get_ending(child_node);
                return Some(IntersectionData { ray: *ray, voxel_data: material });
            } else {

                //let next = chunk.data[voxel_data as usize + child_pop_count(current, true_sub_voxel) as usize];

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

                let result = proc_subtree(ray, chunk, child_node, sub_entry, sub_exit, direction_mask);
                
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

pub fn get_ending(data: u32) -> u32 {
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

fn child_pop_count(data: u32, true_sub_voxel: u32) -> u32 {
    let child_byte = data >> 24;
    let mask = (1 << true_sub_voxel) -1;
    let bits_before = child_byte & mask;
    bits_before.count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_deep_voxel_traversal() {
        // Target Voxel: (25, 6, 18)
        // Depth: 5 levels (32 -> 16 -> 8 -> 4 -> 2 -> 1)
        // L0 (32x32): Child 5 (Right, Bottom, Back)  -> Points to index 1
        // L1 (16x16): Child 1 (Right, Bottom, Front) -> Points to index 2
        // L2 (8x8):   Child 2 (Left,  Top,    Front) -> Points to index 3
        // L3 (4x4):   Child 6 (Left,  Top,    Back)  -> Points to index 4
        // L4 (2x2):   Child 1 (Right, Bottom, Front) -> LEAF! Payload 0xCAFE

        let mut tree_data = vec![0_u32; 5];
        
        // Node 0: Child 5 exists, points to index 1
        tree_data[0] = (1_u32 << (5 + 24)) | 1;
        
        // Node 1: Child 1 exists, points to index 2
        tree_data[1] = (1_u32 << (1 + 24)) | 2;
        
        // Node 2: Child 2 exists, points to index 3
        tree_data[2] = (1_u32 << (2 + 24)) | 3;
        
        // Node 3: Child 6 exists, points to index 4
        tree_data[3] = (1_u32 << (6 + 24)) | 4;
        
        // Node 4: Child 1 exists, IS LEAF, contains payload 0xCAFE
        tree_data[4] = (1_u32 << (1 + 24)) | (1_u32 << (1 + 16)) | 0xCAFE;

        let chunk = Chunk {
            data: tree_data,
            min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
            max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 },
        };

        // Ray starts outside at X=-1, aiming straight along +X into Y=6.5, Z=18.5
        // We use 0.0001 for Y and Z to prevent Float NaN math (a standard engine edge-case fix)
        let ray = Ray {
            origin: V3 { x: -1.0, y: 6.5, z: 18.5 },
            direction: V3 { x: 1.0, y: 0.0001, z: 0.0001 },
        };

        let result = find_intersection(&ray, &chunk, chunk.data[0]); // Start at index 0

        assert!(result.is_some(), "Ray completely missed the deep voxel!");
        if let Some(intersect) = result {
            assert_eq!(intersect.voxel_data, 0xCAFE, "Hit the wrong voxel or extracted wrong data!");
        }
    }

    #[test]
    fn test_standard_cube() {
        // A 2x2x2 cube
        // total_size = 8
        let data: &[&[&[u32]]] = &[
            // x = 0
            &[
                &[1, 0], // y = 0 (z = 0, z = 1)
                &[0, 2], // y = 1 (z = 0, z = 1)
            ],
            // x = 1
            &[
                &[0, 3], // y = 0
                &[4, 0], // y = 1
            ],
        ];

        let result = to_chunks(data);
        
        assert_eq!(result.len(), 8);
        assert_eq!(result, vec![1, 0, 0, 2, 0, 3, 4, 0]);
    }

    #[test]
    fn test_asymmetrical_dimensions() {
        // Width = 1, Height = 2, Depth = 3
        // total_size = 6
        let data: &[&[&[u32]]] = &[
            // x = 0
            &[
                &[1, 2, 3], // y = 0
                &[0, 5, 0], // y = 1
            ]
        ];

        let result = to_chunks(data);
        
        assert_eq!(result.len(), 6);
        assert_eq!(result, vec![1, 2, 3, 0, 5, 0]);
    }

    #[test]
    fn test_single_element() {
        // Width = 1, Height = 1, Depth = 1
        let data: &[&[&[u32]]] = &[
            &[
                &[42]
            ]
        ];

        let result = to_chunks(data);
        
        assert_eq!(result.len(), 1);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_empty_data() {
        // Width = 0
        let data: &[&[&[u32]]] = &[];

        let result = to_chunks(data);
        
        assert_eq!(result.len(), 0);
        assert_eq!(result, Vec::<u32>::new());
    }
}
