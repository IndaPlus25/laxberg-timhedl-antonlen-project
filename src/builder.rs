use crate::octree::*;
use crate::vecmath::*;

use std::collections::HashMap;

use std::collections::VecDeque;

const BRANCH_MARKER: u32 = u32::MAX;
const AIR_MARKER: u32 = 0;

struct SvoChild {
    is_leaf: bool,
    payload: u32,
    pos: V3i,
}

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

/// Standalone function to extract a 32x32x32 flat array from a global coordinate
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

                if global_pos.x >= width || global_pos.y >= height || global_pos.z >= depth {
                    continue;
                }

                let voxel = data[global_pos.x as usize][global_pos.y as usize][global_pos.z as usize];
                
                if voxel == 0 {
                    continue;
                }

                is_empty = false;
                let idx = local_pos.x + (local_pos.y * 32) + (local_pos.z * 1024);
                flat_data[idx as usize] = voxel;
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
    let level_volume = [1, 2, 4, 8, 16, 32];
    
    let mut pyramid: [Vec<u32>; 6] = [
        vec![0; 1],
        vec![0; 8],
        vec![0; 64],
        vec![0; 512],
        vec![0; 4096],
        flat_data.to_vec(),
    ];
    
    // Step 1: Compress the volume from the bottom up
    build_mipmap_pyramid(&mut pyramid, &level_volume);
    
    // Step 2: Write the compressed pyramid into the final 1D SVO format
    serialize_svo_bfs(&pyramid, &level_volume)
}

fn build_mipmap_pyramid(pyramid: &mut [Vec<u32>; 6], volumes: &[i32; 6]) {
    for level in (0..5).rev() { 
        let current_volume = volumes[level];
        let child_volume = volumes[level + 1];
        let child_level = level + 1;
        
        for z in 0..current_volume {
            for y in 0..current_volume {
                for x in 0..current_volume {
                    let pos = V3i { x, y, z };
                    
                    // Abstract away the 8-child check
                    let pruned_value = check_if_prunable(pyramid, child_level, child_volume, &pos);
                    
                    let idx = pos.x + (pos.y * current_volume) + (pos.z * current_volume * current_volume);
                    pyramid[level][idx as usize] = pruned_value;
                }
            }
        }
    }
}

/// Checks the 8 children of a parent node. 
/// Returns a single voxel payload if they are identical, or BRANCH_MARKER if they differ.
fn check_if_prunable(pyramid: &[Vec<u32>; 6], child_level: usize, child_volume: i32, parent_pos: &V3i) -> u32 {
    let mut first_val = None;

    for i in 0..8 {
        let child_pos = V3i {
            x: (parent_pos.x * 2) + (i & 1),
            y: (parent_pos.y * 2) + ((i >> 1) & 1),
            z: (parent_pos.z * 2) + ((i >> 2) & 1),
        };
        
        let child_idx = child_pos.x + (child_pos.y * child_volume) + (child_pos.z * child_volume * child_volume);
        let val = pyramid[child_level][child_idx as usize];

        if val == BRANCH_MARKER { 
            return BRANCH_MARKER;
        }

        if let Some(first) = first_val {
            if first != val {
                return BRANCH_MARKER;
            }
        } else {
            first_val = Some(val);
        }
    }
    
    // If we survived the loop, all 8 children are identical leaves (or all air).
    first_val.unwrap_or(AIR_MARKER)
}

fn serialize_svo_bfs(pyramid: &[Vec<u32>; 6], volumes: &[i32; 6]) -> Vec<u32> {
    let mut out_data: Vec<u32> = Vec::with_capacity(1024);
    out_data.push(0);  

    let mut queue = VecDeque::new();
    queue.push_back((0, V3i { x: 0, y: 0, z: 0 }, 0));
    
    while let Some((level, pos, my_idx)) = queue.pop_front() {
        let child_level = level + 1;
        let child_volume = volumes[child_level];
        
        let (child_mask, leaf_mask, valid_children) = scan_children_for_serialization(
            pyramid, child_level, child_volume, &pos
        );
        
        let pointer = out_data.len() as u32;
        let node_data = (child_mask << 24) | (leaf_mask << 16) | pointer;
        out_data[my_idx] = node_data;
        
        for child in valid_children {
            let child_out_idx = out_data.len();
            
            if child.is_leaf {
                out_data.push(child.payload); 
            } else {
                out_data.push(0);  
                queue.push_back((child_level, child.pos, child_out_idx));
            }
        }
    }
    
    out_data
}

/// Inspects the 8 children, generates the SVO bitmasks, and collects the valid nodes.
fn scan_children_for_serialization(pyramid: &[Vec<u32>; 6], child_level: usize, child_volume: i32, parent_pos: &V3i) -> (u32, u32, Vec<SvoChild>) {
    let mut child_mask: u32 = 0;
    let mut leaf_mask: u32 = 0;
    let mut valid_children = Vec::with_capacity(8);
    
    for i in 0..8 {
        let child_pos = V3i {
            x: (parent_pos.x * 2) + (i & 1),
            y: (parent_pos.y * 2) + ((i >> 1) & 1),
            z: (parent_pos.z * 2) + ((i >> 2) & 1),
        };
        
        let child_idx = child_pos.x + (child_pos.y * child_volume) + (child_pos.z * child_volume * child_volume);
        let val = pyramid[child_level][child_idx as usize];
        
        if val != AIR_MARKER { 
            child_mask |= 1 << i;
            
            if val != BRANCH_MARKER { 
                leaf_mask |= 1 << i;
                valid_children.push(SvoChild { is_leaf: true, payload: val, pos: child_pos });
            } else { 
                valid_children.push(SvoChild { is_leaf: false, payload: val, pos: child_pos });
            }
        }
    }
    
    (child_mask, leaf_mask, valid_children)
}
