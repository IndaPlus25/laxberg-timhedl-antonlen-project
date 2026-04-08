use crate::octree::*;
use crate::vecmath::*;

use std::collections::HashMap;

pub fn to_chunks(data: &[&[&[u32]]]) -> HashMap<V3i, Chunk> {
    let width = data.len();
    let height = if width > 0 { data[0].len() } else { 0 };
    let depth = if height > 0 { data[0][0].len() } else { 0 };

    let mut chunks = HashMap::new();

    // Group the chunk counts into a V3i for cleaner management
    let chunk_counts = V3i {
        x: (width as f32 / 32.0).ceil() as i32,
        y: (height as f32 / 32.0).ceil() as i32,
        z: (depth as f32 / 32.0).ceil() as i32,
    };

    for cx in 0..chunk_counts.x {
        for cy in 0..chunk_counts.y {
            for cz in 0..chunk_counts.z {
                let chunk_pos = V3i { x: cx, y: cy, z: cz };
                
                // Delegate the heavy lifting to the standalone extractor
                if let Some(flat_data) = extract_hot_chunk(data, &chunk_pos) {
                    
                    // Build the SVO
                    let chunk_tree_data = build_chunk(&flat_data);
                    
                    let min_pos = V3 { 
                        x: (chunk_pos.x * 32) as f32, 
                        y: (chunk_pos.y * 32) as f32, 
                        z: (chunk_pos.z * 32) as f32 
                    };
                    let max_pos = V3 { 
                        x: ((chunk_pos.x + 1) * 32) as f32, 
                        y: ((chunk_pos.y + 1) * 32) as f32, 
                        z: ((chunk_pos.z + 1) * 32) as f32 
                    };

                    chunks.insert(chunk_pos, Chunk {
                        data: chunk_tree_data,
                        min_pos,
                        max_pos,
                    });
                }
            }
        }
    }

    chunks
}

pub fn extract_hot_chunk(data: &[&[&[u32]]], chunk_pos: &V3i) -> Option<Vec<u32>> {
    let width = data.len() as i32;
    let height = if width > 0 { data[0].len() as i32 } else { 0 };
    let depth = if height > 0 { data[0][0].len() as i32 } else { 0 };

    let mut flat_data = vec![0_u32; 32768];
    let mut is_empty = true;

    for lx in 0..32 {
        for ly in 0..32 {
            for lz in 0..32 {
                let local_pos = V3i { x: lx, y: ly, z: lz };
                
                let global_pos = V3i {
                    x: (chunk_pos.x * 32) + local_pos.x,
                    y: (chunk_pos.y * 32) + local_pos.y,
                    z: (chunk_pos.z * 32) + local_pos.z,
                };

                // Bounds check against the main data array
                if global_pos.x < width && global_pos.y < height && global_pos.z < depth {
                    let voxel = data[global_pos.x as usize][global_pos.y as usize][global_pos.z as usize];
                    
                    if voxel != 0 {
                        is_empty = false;
                        let idx = local_pos.x + (local_pos.y * 32) + (local_pos.z * 1024);
                        flat_data[idx as usize] = voxel;
                    }
                }
            }
        }
    }

    if is_empty {
        None
    } else {
        Some(flat_data)
    }
}

pub fn build_chunk(flat_data: &[u32]) -> Vec<u32> {
    // Represents the side length of the volume at each depth level.
    let volumes = [1, 2, 4, 8, 16, 32];
    
    let mut pyramid: [Vec<u32>; 6] = [
        vec![0; 1],
        vec![0; 8],
        vec![0; 64],
        vec![0; 512],
        vec![0; 4096],
        flat_data.to_vec(),
    ];
    
    // --- PHASE 1: BOTTOM-UP MIPMAP PRUNING ---
    for level in (0..5).rev() { 
        let current_vol = volumes[level];
        let child_vol = volumes[level + 1];
        
        for z in 0..current_vol {
            for y in 0..current_vol {
                for x in 0..current_vol {
                    let pos = V3i { x, y, z };
                    
                    let mut all_identical = true;
                    let mut first_val = None;
                    
                    for i in 0..8 {
                        let child_pos = V3i {
                            x: (pos.x * 2) + (i & 1),
                            y: (pos.y * 2) + ((i >> 1) & 1),
                            z: (pos.z * 2) + ((i >> 2) & 1),
                        };
                        
                        let child_idx = child_pos.x + (child_pos.y * child_vol) + (child_pos.z * child_vol * child_vol);
                        let val = pyramid[level + 1][child_idx as usize];
                        
                        if val == u32::MAX { 
                            all_identical = false;
                            break;
                        }
                        
                        if let Some(first) = first_val {
                            if first != val {
                                all_identical = false;
                                break;
                            }
                        } else {
                            first_val = Some(val);
                        }
                    }
                    
                    let idx = pos.x + (pos.y * current_vol) + (pos.z * current_vol * current_vol);
                    
                    if all_identical {
                        pyramid[level][idx as usize] = first_val.unwrap_or(0);
                    } else {
                        pyramid[level][idx as usize] = u32::MAX;
                    }
                }
            }
        }
    }
    
    // --- PHASE 2: TOP-DOWN BFS SERIALIZATION ---
    let mut out_data: Vec<u32> = Vec::with_capacity(1024);
    out_data.push(0); 
    
    // Queue stores: (level, V3i_position, parent_index_in_out_data)
    let mut queue = std::collections::VecDeque::new();
    queue.push_back((0, V3i { x: 0, y: 0, z: 0 }, 0));
    
    while let Some((level, pos, my_idx)) = queue.pop_front() {
        let mut child_mask: u32 = 0;
        let mut leaf_mask: u32 = 0;
        
        // Tuple: (is_leaf, voxel_payload, child_V3i_position)
        let mut valid_children = Vec::with_capacity(8);
        
        let child_vol = volumes[level + 1];
        
        for i in 0..8 {
            let child_pos = V3i {
                x: (pos.x * 2) + (i & 1),
                y: (pos.y * 2) + ((i >> 1) & 1),
                z: (pos.z * 2) + ((i >> 2) & 1),
            };
            
            let child_idx = child_pos.x + (child_pos.y * child_vol) + (child_pos.z * child_vol * child_vol);
            let val = pyramid[level + 1][child_idx as usize];
            
            if val != 0 { 
                child_mask |= 1 << i;
                
                if val != u32::MAX { 
                    leaf_mask |= 1 << i;
                    valid_children.push((true, val, child_pos));
                } else { 
                    valid_children.push((false, val, child_pos));
                }
            }
        }
        
        let pointer = out_data.len() as u32;
        
        let node_data = (child_mask << 24) | (leaf_mask << 16) | pointer;
        out_data[my_idx] = node_data;
        
        // Unpacking the clean tuple
        for (is_leaf, val, child_pos) in valid_children {
            let child_out_idx = out_data.len();
            
            if is_leaf {
                out_data.push(val); 
            } else {
                out_data.push(0); 
                queue.push_back((level + 1, child_pos, child_out_idx));
            }
        }
    }
    
    out_data
}

