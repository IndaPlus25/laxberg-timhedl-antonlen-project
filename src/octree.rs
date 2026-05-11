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

impl Chunk {
    /// Recursively traverses and merges nodes. 
    /// Appends new child blocks to `self.data` if node boundaries change.
    fn merge_node_in_place(
        &mut self,
        self_val: u32,
        self_is_leaf: bool,
        other_data: &[u32],
        other_val: u32,
        other_is_leaf: bool
    ) -> (bool, u32) {
        // SVO Optimization: If 'other' is a solid leaf block, it completely overwrites 'self'
        // without needing to traverse or allocate children further down.
        if other_is_leaf {
            return (true, other_val);
        }

        let mut s_val = self_val;
        let mut s_is_leaf = self_is_leaf;

        // If self is a solid leaf but 'other' specifies sub-details, we must subdivide self 
        // into a parent with 8 leaf children so we can merge the partial 'other' data into it.
        if s_is_leaf {
            let new_pointer = self.data.len() as u32;
            self.data.extend(std::iter::repeat(s_val).take(8));
            s_val = (0xFF << 24) | (0xFF << 16) | new_pointer;
            s_is_leaf = false;
        }

        let s_pointer = get_ending(s_val);
        let o_pointer = get_ending(other_val);

        let mut new_cc: u32 = 0;
        let mut new_ll: u32 = 0;

        let mut merged_children: [Option<(bool, u32)>; 8] = [None; 8];
        let mut mask_changed = false;
        let mut children_count = 0;

        for i in 0..8 {
            let s_has = has_child(s_val, i);
            let o_has = has_child(other_val, i);

            if !s_has && !o_has {
                continue;
            }

            if !s_has && o_has {
                // Self is empty here, but other has data. Deep copy other's subtree into self.
                mask_changed = true;
                let o_child_is_leaf = is_leaf(other_val, i);
                let o_idx = o_pointer as usize + child_pop_count(other_val, i) as usize;
                
                let copied_child = self.copy_subtree_from_other(other_data, other_data[o_idx], o_child_is_leaf);
                merged_children[i as usize] = Some(copied_child);
                
                new_cc |= 1 << i;
                if copied_child.0 { new_ll |= 1 << i; }
                children_count += 1;

            } else if s_has && !o_has {
                // SVO Optimization: Other is empty here. Keep self's un-mutated child.
                let s_child_is_leaf = is_leaf(s_val, i);
                let s_idx = s_pointer as usize + child_pop_count(s_val, i) as usize;
                
                merged_children[i as usize] = Some((s_child_is_leaf, self.data[s_idx]));
                
                new_cc |= 1 << i;
                if s_child_is_leaf { new_ll |= 1 << i; }
                children_count += 1;

            } else {
                // Both have data, we must recurse and merge them.
                let s_child_is_leaf = is_leaf(s_val, i);
                let s_idx = s_pointer as usize + child_pop_count(s_val, i) as usize;
                let o_child_is_leaf = is_leaf(other_val, i);
                let o_idx = o_pointer as usize + child_pop_count(other_val, i) as usize;

                let s_child_val = self.data[s_idx];
                let o_child_val = other_data[o_idx];

                let merged_child = self.merge_node_in_place(
                    s_child_val, s_child_is_leaf,
                    other_data, o_child_val, o_child_is_leaf
                );

                if merged_child.0 != s_child_is_leaf || merged_child.1 != s_child_val {
                    mask_changed = true;
                }

                merged_children[i as usize] = Some(merged_child);
                new_cc |= 1 << i;
                if merged_child.0 { new_ll |= 1 << i; }
                children_count += 1;
            }
        }

        // SVO Optimization: Try to collapse 8 identical solid leaf blocks back into a single leaf.
        if children_count == 8 {
            let mut all_same_leaf = true;
            let mut first_payload = 0;
            for i in 0..8 {
                if let Some((true, p)) = merged_children[i] {
                    if i == 0 { first_payload = p; }
                    else if p != first_payload { all_same_leaf = false; break; }
                } else {
                    all_same_leaf = false;
                    break;
                }
            }
            if all_same_leaf {
                return (true, first_payload);
            }
        }

        let original_count = (s_val >> 24).count_ones();
        if children_count != original_count {
            mask_changed = true;
        }

        let mut final_pointer = s_pointer;

        // If the size/structure changed, we allocate a new contiguous block for the children.
        // If it didn't change, we modify the existing block directly in place to save memory.
        if mask_changed {
            final_pointer = self.data.len() as u32;
            for _ in 0..children_count {
                self.data.push(0);
            }
        }

        let mut idx = 0;
        for i in 0..8 {
            if let Some((_, val)) = merged_children[i] {
                self.data[final_pointer as usize + idx] = val;
                idx += 1;
            }
        }

        let new_node = (new_cc << 24) | (new_ll << 16) | final_pointer;
        (false, new_node)
    }

    /// Recursively flattens and copies a subtree from `other` into `self.data`.
    fn copy_subtree_from_other(&mut self, other_data: &[u32], val: u32, is_leaf: bool) -> (bool, u32) {
        if is_leaf {
            return (true, val);
        }

        let pointer = get_ending(val);
        let cc = val >> 24;
        let ll = (val >> 16) & 0xFF;
        
        let pop_count = cc.count_ones() as usize;
        let new_pointer = self.data.len() as u32;
        
        // Reserve space for children
        for _ in 0..pop_count {
            self.data.push(0); 
        }

        let mut idx = 0;
        for i in 0..8 {
            if (cc & (1 << i)) != 0 {
                let child_is_leaf = (ll & (1 << i)) != 0;
                let child_val = other_data[pointer as usize + idx];
                let copied = self.copy_subtree_from_other(other_data, child_val, child_is_leaf);
                self.data[new_pointer as usize + idx] = copied.1;
                idx += 1;
            }
        }

        (false, (cc << 24) | (ll << 16) | new_pointer)
    }
}
//32x32x32 non-octree optimized chunk, raw data
pub struct FlatChunk {

    pub data: Vec<u32>,
    pub min_pos: V3,
    pub max_pos: V3,

}



pub fn cast_ray(ray: &Ray, chunks: &HashMap<V3i, Chunk>, limit: u32) -> Option<IntersectionData> {
    let chunk_size = 32.0;

    let origin_scaled = vec_div_scal(&ray.origin, chunk_size);
    let mut chunk_pos = vec_floor_to_v3i(&origin_scaled);
    let inv_dir = vec_inv_dir_dda(&ray.direction);

    let mut step = V3i { x: 0, y: 0, z: 0 };
    let mut t_max = V3 { x: 0.0, y: 0.0, z: 0.0 };
    let mut t_delta = V3 { x: 0.0, y: 0.0, z: 0.0 };

    if ray.direction.x.abs() < f32::EPSILON {
        step.x = 0;
        t_delta.x = f32::INFINITY;
        t_max.x = f32::INFINITY;
    } else {
        t_delta.x = (chunk_size * inv_dir.x).abs();
        if ray.direction.x > 0.0 {
            step.x = 1;
            t_max.x = (((chunk_pos.x + 1) as f32 * chunk_size) - ray.origin.x) * inv_dir.x;
        } else {
            step.x = -1;
            t_max.x = (ray.origin.x - (chunk_pos.x as f32 * chunk_size)) * -inv_dir.x;
        }
    }

    if ray.direction.y.abs() < f32::EPSILON {
        step.y = 0;
        t_delta.y = f32::INFINITY;
        t_max.y = f32::INFINITY;
    } else {
        t_delta.y = (chunk_size * inv_dir.y).abs();
        if ray.direction.y > 0.0 {
            step.y = 1;
            t_max.y = (((chunk_pos.y + 1) as f32 * chunk_size) - ray.origin.y) * inv_dir.y;
        } else {
            step.y = -1;
            t_max.y = (ray.origin.y - (chunk_pos.y as f32 * chunk_size)) * -inv_dir.y;
        }
    }

    if ray.direction.z.abs() < f32::EPSILON {
        step.z = 0;
        t_delta.z = f32::INFINITY;
        t_max.z = f32::INFINITY;
    } else {
        t_delta.z = (chunk_size * inv_dir.z).abs();
        if ray.direction.z > 0.0 {
            step.z = 1;
            t_max.z = (((chunk_pos.z + 1) as f32 * chunk_size) - ray.origin.z) * inv_dir.z;
        } else {
            step.z = -1;
            t_max.z = (ray.origin.z - (chunk_pos.z as f32 * chunk_size)) * -inv_dir.z;
        }
    }

   for _ in 0..limit {
        
        if let Some(chunk) = chunks.get(&chunk_pos) {
            if !chunk.data.is_empty() {
                let root_data = chunk.data[0]; 
                
                if let Some(hit) = find_intersection(ray, chunk, root_data) {
                    return Some(hit);
                }
            }
        }

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

fn proc_subtree(ray: &Ray, chunk: &Chunk, current: u32, entry: V3, exit: V3, direction_mask: u32) -> Option<IntersectionData> {
    
    let mid = vec_mult_scal(&vec_add(&entry, &exit), 0.5);
    
    let t_min = entry.x.max(entry.y).max(entry.z);

    let mut first_child_intersect: u32 = 0; 

    if t_min < 0.0 {
        if mid.x < 0.0 { first_child_intersect |= 1; }
        if mid.y < 0.0 { first_child_intersect |= 2; }
        if mid.z < 0.0 { first_child_intersect |= 4; }
    } else {
        let entry_plane = vec_entry_plane(&entry);
        if entry_plane == 0 {
            if mid.y < entry.x { first_child_intersect |= 2; }
            if mid.z < entry.x { first_child_intersect |= 4; }
        } else if entry_plane == 1 {
            if mid.x < entry.y { first_child_intersect |= 1; }
            if mid.z < entry.y { first_child_intersect |= 4; }
        } else {
            if mid.x < entry.z { first_child_intersect |= 1; }
            if mid.y < entry.z { first_child_intersect |= 2; }
        }
    }

    let mut current_sub_voxel: u32 = first_child_intersect;

    loop {


        let true_sub_voxel: u32 = current_sub_voxel ^ direction_mask;

        if has_child(current, true_sub_voxel) {
            let pointer = get_ending(current);
            let child_index = pointer as usize + child_pop_count(current, true_sub_voxel) as usize;
            if child_index >= chunk.data.len() {
                return None;
            }

            let node_at_index = chunk.data[child_index];

            if is_leaf(current, true_sub_voxel) {
                let material = get_ending(node_at_index);
                return Some(IntersectionData { ray: *ray, voxel_data: material });
            } else {
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

                let result = proc_subtree(ray, chunk, node_at_index, sub_entry, sub_exit, direction_mask);
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
mod macro_traversal_tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_chunk(cx: i32, cy: i32, cz: i32, payload: u32) -> Chunk {
        let mut root_node_data: u32 = 0;
        root_node_data |= 1 << 24; // Child 0 exists 
        root_node_data |= 1 << 16; // Child 0 is leaf
        root_node_data |= 1;       // Pointer to index 1

        Chunk {
            // Data now correctly holds the root pointer AND the leaf payload
            data: vec![root_node_data, payload],
            min_pos: V3 { x: (cx * 32) as f32, y: (cy * 32) as f32, z: (cz * 32) as f32 },
            max_pos: V3 { x: ((cx + 1) * 32) as f32, y: ((cy + 1) * 32) as f32, z: ((cz + 1) * 32) as f32 },
        }
    }

    #[test]
    fn test_macro_dda_positive_axis() {
        let mut chunks = HashMap::new();
        chunks.insert(V3i { x: 3, y: 0, z: 0 }, create_test_chunk(3, 0, 0, 0x1111));

        let ray = Ray {
            origin: V3 { x: 1.0, y: 1.0, z: 1.0 }, 
            direction: V3 { x: 1.0, y: 0.0, z: 0.0 }, 
        };

        let result = cast_ray(&ray, &chunks, 10);
        
        assert!(result.is_some(), "Ray failed to cross empty chunks to hit target.");
        if let Some(hit) = result {
            assert_eq!(hit.voxel_data, 0x1111, "Ray hit something, but not the target chunk data.");
        }
    }

    #[test]
    fn test_macro_dda_ray_limit() {
        let mut chunks = HashMap::new();
        chunks.insert(V3i { x: 5, y: 0, z: 0 }, create_test_chunk(5, 0, 0, 0x2222));

        let ray = Ray {
            origin: V3 { x: 1.0, y: 1.0, z: 1.0 }, 
            direction: V3 { x: 1.0, y: 0.0, z: 0.0 }, 
        };

        let result = cast_ray(&ray, &chunks, 3);
        assert!(result.is_none(), "Ray should have stopped due to step limit, but continued.");
    }

    #[test]
    fn test_macro_dda_negative_diagonal() {
        let mut chunks = HashMap::new();
        
        let mut root_node_data: u32 = 0;
        root_node_data |= 1_u32 << 31; // Child 7 exists
        root_node_data |= 1 << 23;     // Child 7 is leaf
        root_node_data |= 1;           // Pointer to index 1

        let target_chunk = Chunk {
            data: vec![root_node_data, 0x3333], // Correctly structured payload
            min_pos: V3 { x: -64.0, y: -64.0, z: -64.0 }, 
            max_pos: V3 { x: -32.0, y: -32.0, z: -32.0 },
        };
        chunks.insert(V3i { x: -2, y: -2, z: -2 }, target_chunk);

        let ray = Ray {
            origin: V3 { x: 40.0, y: 40.0, z: 40.0 }, 
            direction: V3 { x: -0.577, y: -0.577, z: -0.577 }, 
        };

        let result = cast_ray(&ray, &chunks, 20);
        
        assert!(result.is_some(), "Failed to traverse diagonally backwards across chunk boundaries.");
        if let Some(hit) = result {
            assert_eq!(hit.voxel_data, 0x3333);
        }
    }

    #[test]
    fn test_macro_dda_grazing_edge() {
        let mut chunks = HashMap::new();
        chunks.insert(V3i { x: 1, y: 1, z: 0 }, create_test_chunk(1, 1, 0, 0x4444));

        let ray = Ray {
            origin: V3 { x: 0.1, y: 0.1, z: 1.0 }, 
            direction: V3 { x: std::f32::consts::FRAC_1_SQRT_2 , y: std::f32::consts::FRAC_1_SQRT_2, z: 0.0 }, 
        };

        let result = cast_ray(&ray, &chunks, 5);
        assert!(result.is_some(), "Ray grazing a chunk corner failed to enter the adjacent chunk.");
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
        let mut root_node_data: u32 = 0;
        root_node_data |= 1 << 24;    // Child 0 exists
        root_node_data |= 1 << 16;    // Child 0 is leaf
        // POINTER: Point to index 1 in the data array
        root_node_data |= 1;          

        // LEAF NODE: The actual child occupying a slot, holding the payload
        let leaf_payload: u32 = 0x9999;     

        let chunk = Chunk {
            data: vec![root_node_data, leaf_payload], // Vector now has length 2
            min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
            max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 },
        };

        let ray = Ray {
            origin: V3 { x: -5.0, y: 8.0, z: 8.0 }, 
            direction: V3 { x: 1.0, y: 0.0, z: 0.0 },
        };

        let result = find_intersection(&ray, &chunk, root_node_data);
        
        assert!(result.is_some(), "Ray should have hit voxel 0");
        if let Some(intersect) = result {
            assert_eq!(intersect.voxel_data, 0x9999, "Returned incorrect payload"); 
        }
    }

    #[test]
    fn test_negative_ray_reflection() {
        let mut root_node_data: u32 = 0;
        root_node_data |= 1_u32 << 31; // Child 7 exists 
        root_node_data |= 1 << 23;     // Child 7 is leaf
        // POINTER: Point to index 1 in the data array
        root_node_data |= 1;           

        // LEAF NODE: The payload
        let leaf_payload: u32 = 0x7777;      

        let chunk = Chunk {
            data: vec![root_node_data, leaf_payload], // Vector now has length 2
            min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
            max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 },
        };

        let ray = Ray {
            origin: V3 { x: 40.0, y: 40.0, z: 40.0 }, 
            direction: V3 { x: -0.577, y: -0.577, z: -0.577 }, 
        };

        let result = find_intersection(&ray, &chunk, root_node_data);
        
        assert!(result.is_some(), "Negative ray reflection failed to hit voxel 7");
        if let Some(intersect) = result {
            assert_eq!(intersect.voxel_data, 0x7777, "Hit the right voxel, but got the wrong data");
        }
    }

    #[test]
    fn test_deep_voxel_traversal() {
        // We now need 6 elements, because Node 4 points to Node 5
        let mut tree_data = vec![0_u32; 6]; 
        
        // Node 0: Child 5 exists, points to index 1
        tree_data[0] = (1_u32 << (5 + 24)) | 1;
        
        // Node 1: Child 1 exists, points to index 2
        tree_data[1] = (1_u32 << (1 + 24)) | 2;
        
        // Node 2: Child 2 exists, points to index 3
        tree_data[2] = (1_u32 << (2 + 24)) | 3;
        
        // Node 3: Child 6 exists, points to index 4
        tree_data[3] = (1_u32 << (6 + 24)) | 4;
        
        // Node 4: Child 1 exists, IS LEAF, points to index 5
        tree_data[4] = (1_u32 << (1 + 24)) | (1_u32 << (1 + 16)) | 5;

        // Node 5: The actual leaf node, contains payload 0xCAFE
        tree_data[5] = 0xCAFE;

        let chunk = Chunk {
            data: tree_data,
            min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
            max_pos: V3 { x: 32.0, y: 32.0, z: 32.0 },
        };

        let ray = Ray {
            origin: V3 { x: -1.0, y: 6.5, z: 18.5 },
            direction: V3 { x: 1.0, y: 0.0001, z: 0.0001 },
        };

        let result = find_intersection(&ray, &chunk, chunk.data[0]); 

        assert!(result.is_some(), "Ray completely missed the deep voxel!");
        if let Some(intersect) = result {
            assert_eq!(intersect.voxel_data, 0xCAFE, "Hit the wrong voxel or extracted wrong data!");
        }
    } 
}
