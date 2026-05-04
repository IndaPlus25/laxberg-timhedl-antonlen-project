// ==========================================
// 1. DATASTRUKTURER & BIND GROUPS
// ==========================================

struct Camera {
    position: vec3<f32>,
    render_distance: u32,
    top_left: vec3<f32>,
    delta_x: vec3<f32>,
    delta_y: vec3<f32>,
}

struct HitData {
    hit_pos: vec3<f32>,     
    did_hit: u32,          
    hit_normal: vec3<f32>, 
    payload: u32,          
}

struct Lighting {
    sun_direction: vec3<f32>,
    ambient_strength: f32,
    face_multipliers_1: vec4<f32>, 
    face_multipliers_2: vec4<f32>, 
    sky_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var screen_texture: texture_storage_2d<bgra8unorm, write>;
@group(0) @binding(2) var<storage, read> world_data: array<u32>; 
@group(0) @binding(3) var<storage, read> colour_lut: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> hit_buffer: array<HitData>;
@group(0) @binding(5) var<uniform> lighting: Lighting;


// ==========================================
// 2. HJÄLPFUNKTIONER (vecmath.rs)
// ==========================================

fn vec_entry_plane(v: vec3<f32>) -> u32 {
    if (v.x >= v.y && v.x >= v.z) { return 0u; } // YZ plane
    else if (v.y >= v.x && v.y >= v.z) { return 1u; } // XZ plane
    else { return 2u; } // XY plane
}

fn vec_exit_plane(v: vec3<f32>) -> u32 {
    if (v.x <= v.y && v.x <= v.z) { return 0u; } // YZ
    else if (v.y <= v.x && v.y <= v.z) { return 1u; } // XZ
    else { return 2u; } // XY
}

fn get_face_multiplier(normal: vec3<f32>) -> f32 {
    if (normal.x > 0.5) { return lighting.face_multipliers_1.x; }
    if (normal.x < -0.5) { return lighting.face_multipliers_1.y; }
    if (normal.y > 0.5) { return lighting.face_multipliers_1.z; }
    if (normal.y < -0.5) { return lighting.face_multipliers_1.w; }
    if (normal.z > 0.5) { return lighting.face_multipliers_2.x; }
    return lighting.face_multipliers_2.y; 
}

// ==========================================
// 3. BITWISE FUNKTIONER (octree.rs)
// ==========================================

fn get_ending(data: u32) -> u32 {
    return data & 0xFFFFu;
}

fn is_leaf(data: u32, position: u32) -> bool {
    let n = 1u << (position + 16u);
    return (data & n) != 0u;
}

fn has_child(data: u32, position: u32) -> bool {
    let n = 1u << (position + 24u);
    return (data & n) != 0u;
}

fn child_pop_count(data: u32, true_sub_voxel: u32) -> u32 {
    let child_byte = data >> 24u;
    let mask = (1u << true_sub_voxel) - 1u;
    let bits_before = child_byte & mask;
    return countOneBits(bits_before);
}

// ==========================================
// 4. MIKRO-TRAVERSERING (proc_subtree Stack)
// ==========================================

struct StackFrame {
    node_data: u32,
    sub_voxel: u32,
}

fn get_first_child_intersect(t_min: f32, entry: vec3<f32>, mid: vec3<f32>) -> u32 {
    var first_child: u32 = 0u;
    if (t_min < 0.0) {
        if (mid.x < 0.0) { first_child |= 1u; }
        if (mid.y < 0.0) { first_child |= 2u; }
        if (mid.z < 0.0) { first_child |= 4u; }
    } else {
        let entry_plane = vec_entry_plane(entry);
        if (entry_plane == 0u) {
            if (mid.y < entry.x) { first_child |= 2u; }
            if (mid.z < entry.x) { first_child |= 4u; }
        } else if (entry_plane == 1u) {
            if (mid.x < entry.y) { first_child |= 1u; }
            if (mid.z < entry.y) { first_child |= 4u; }
        } else {
            if (mid.x < entry.z) { first_child |= 1u; }
            if (mid.y < entry.z) { first_child |= 2u; }
        }
    }
    return first_child;
}

// improved Revelles et al. transition tabell 
fn get_next_sub_voxel(current: u32, exit_plane: u32) -> u32 {    
    let lut = array<u32, 3>(
        0x87858381u, // Plane 0 (YZ): [8, 7, 8, 5, 8, 3, 8, 1]
        0x88768832u, // Plane 1 (XZ): [8, 8, 7, 6, 8, 8, 3, 2]
        0x88887654u  // Plane 2 (XY): [8, 8, 8, 8, 7, 6, 5, 4]
    );

    let plane_data = lut[exit_plane];

    let shift = current * 4u;
    
    return (plane_data >> shift) & 0xFu;
}

//Optimised for LOW warp divergence (camera rays)
fn find_intersection(ray_origin: vec3<f32>, ray_dir: vec3<f32>, chunk_min: vec3<f32>, chunk_max: vec3<f32>, chunk_offset: u32, out_payload: ptr<function, u32>, out_pos: ptr<function, vec3<f32>>, out_normal: ptr<function, vec3<f32>>) -> bool {
    var direction_mask: u32 = 0u;
    var pos_ray_dir = ray_dir;
    var pos_ray_origin = ray_origin;

    if (pos_ray_dir.x < 0.0) { direction_mask |= 1u; pos_ray_dir.x = -pos_ray_dir.x; pos_ray_origin.x = chunk_max.x - (ray_origin.x - chunk_min.x); }
    if (pos_ray_dir.y < 0.0) { direction_mask |= 2u; pos_ray_dir.y = -pos_ray_dir.y; pos_ray_origin.y = chunk_max.y - (ray_origin.y - chunk_min.y); }
    if (pos_ray_dir.z < 0.0) { direction_mask |= 4u; pos_ray_dir.z = -pos_ray_dir.z; pos_ray_origin.z = chunk_max.z - (ray_origin.z - chunk_min.z); }

    let safe_dir = max(pos_ray_dir, vec3<f32>(0.0000001));
    let pos_inv_dir = vec3<f32>(1.0) / safe_dir;

    let entry = (chunk_min - pos_ray_origin) * pos_inv_dir;
    let exit = (chunk_max - pos_ray_origin) * pos_inv_dir;

    let t_min = max(entry.x, max(entry.y, entry.z));
    let t_max = min(exit.x, min(exit.y, exit.z));

    if (t_min >= t_max || t_max < 0.0) { return false; }

    var stack: array<u32, 5>;

    var stack_subs: u32 = 0u;
    var sp: i32 = 0;

    let root_node_data = world_data[chunk_offset];
    
    var current_center = (chunk_min + chunk_max) * 0.5;
    var current_half_size = (chunk_max.x - chunk_min.x) * 0.5; 

    let mid = (entry + exit) * 0.5;
    let root_sub_voxel = get_first_child_intersect(t_min, entry, mid);
    
    stack_subs = root_sub_voxel;
    stack[0] = root_node_data;
    
    while (sp >= 0) {
        let shift = u32(sp) * 6u;

        // Read current frame
        var current_node = stack[sp];
        var raw_sub = (stack_subs >> shift) & 0x3Fu;
        
        let visited = (raw_sub & 16u) != 0u;
        let actual_sub = raw_sub & 15u; 

        // ASCEND
        if (actual_sub > 7u) {
            sp--; 
            if (sp >= 0) {
                let parent_shift = u32(sp) * 6u;
                let parent_sub = (stack_subs >> parent_shift) & 15u;

                current_center.x -= select(-current_half_size, current_half_size, (parent_sub & 1u) != 0u);
                current_center.y -= select(-current_half_size, current_half_size, (parent_sub & 2u) != 0u);
                current_center.z -= select(-current_half_size, current_half_size, (parent_sub & 4u) != 0u);
                current_half_size *= 2.0;
            }
            continue;
        }

        let voxel_min = current_center - vec3<f32>(current_half_size);
        let voxel_max = current_center + vec3<f32>(current_half_size);
        let f_entry = (voxel_min - pos_ray_origin) * pos_inv_dir;
        let f_exit = (voxel_max - pos_ray_origin) * pos_inv_dir;
        let f_mid = (f_entry + f_exit) * 0.5;

        let true_sub_voxel = actual_sub ^ direction_mask;
        let child_exists = has_child(current_node, true_sub_voxel);
        
        // DECEND
        if (!visited && child_exists) {
            let pointer = get_ending(current_node);
            let child_index = pointer + child_pop_count(current_node, true_sub_voxel);
            let node_at_index = world_data[chunk_offset + child_index];

            if (is_leaf(current_node, true_sub_voxel)) {
                *out_payload = get_ending(node_at_index);

                let leaf_min_mirrored = vec3<f32>(
                    select(voxel_min.x, current_center.x, (actual_sub & 1u) != 0u),
                    select(voxel_min.y, current_center.y, (actual_sub & 2u) != 0u),
                    select(voxel_min.z, current_center.z, (actual_sub & 4u) != 0u)
                );


                let tnear = (leaf_min_mirrored - pos_ray_origin) * pos_inv_dir;
                let t_hit = max(max(tnear.x, tnear.y), tnear.z);
                
                *out_pos = ray_origin + ray_dir * t_hit;
                
                var normal = vec3<f32>(0.0);
                if (tnear.x >= tnear.y && tnear.x >= tnear.z) { 
                    normal = vec3<f32>(-sign(ray_dir.x), 0.0, 0.0); 
                } else if (tnear.y >= tnear.x && tnear.y >= tnear.z) { 
                    normal = vec3<f32>(0.0, -sign(ray_dir.y), 0.0); 
                } else { 
                    normal = vec3<f32>(0.0, 0.0, -sign(ray_dir.z)); 
                }
                *out_normal = normal;                

                return true;
            }

            stack_subs |= (16u << shift);
            
            current_half_size *= 0.5;
            current_center.x += select(-current_half_size, current_half_size, (actual_sub & 1u) != 0u);
            current_center.y += select(-current_half_size, current_half_size, (actual_sub & 2u) != 0u);
            current_center.z += select(-current_half_size, current_half_size, (actual_sub & 4u) != 0u);

            let child_voxel_min = current_center - vec3<f32>(current_half_size);
            let child_voxel_max = current_center + vec3<f32>(current_half_size);
            let sub_entry = (child_voxel_min - pos_ray_origin) * pos_inv_dir;
            let sub_exit = (child_voxel_max - pos_ray_origin) * pos_inv_dir;
            
            let child_t_min = max(sub_entry.x, max(sub_entry.y, sub_entry.z));
            let child_mid = (sub_entry + sub_exit) * 0.5;

            sp++;
            let new_shift = u32(sp) * 6u;
            let new_sub = get_first_child_intersect(child_t_min, sub_entry, child_mid);

            stack[sp] = node_at_index;
            stack_subs = (stack_subs & ~(0x3Fu << new_shift)) | (new_sub << new_shift);

            continue;
        }

        let node_exit = vec3<f32>(
            select(f_mid.x, f_exit.x, (actual_sub & 1u) != 0u),
            select(f_mid.y, f_exit.y, (actual_sub & 2u) != 0u),
            select(f_mid.z, f_exit.z, (actual_sub & 4u) != 0u)
        );

        let next_sub = get_next_sub_voxel(actual_sub, vec_exit_plane(node_exit));
        stack_subs = (stack_subs & ~(0x3Fu << shift)) | (next_sub << shift);

    }
    
    return false;
}

// ----------------------------------------------------
// ANY-HIT TRAVERSAL (For Shadow Rays)
// ----------------------------------------------------

fn find_intersection_anyhit(ray_origin: vec3<f32>, ray_dir: vec3<f32>, chunk_min: vec3<f32>, chunk_max: vec3<f32>, chunk_offset: u32) -> bool {
    var direction_mask: u32 = 0u;
    var pos_ray_dir = ray_dir;
    var pos_ray_origin = ray_origin;

    if (pos_ray_dir.x < 0.0) { direction_mask |= 1u; pos_ray_dir.x = -pos_ray_dir.x; pos_ray_origin.x = chunk_max.x - (ray_origin.x - chunk_min.x); }
    if (pos_ray_dir.y < 0.0) { direction_mask |= 2u; pos_ray_dir.y = -pos_ray_dir.y; pos_ray_origin.y = chunk_max.y - (ray_origin.y - chunk_min.y); }
    if (pos_ray_dir.z < 0.0) { direction_mask |= 4u; pos_ray_dir.z = -pos_ray_dir.z; pos_ray_origin.z = chunk_max.z - (ray_origin.z - chunk_min.z); }

    let safe_dir = max(pos_ray_dir, vec3<f32>(0.0000001));
    let pos_inv_dir = vec3<f32>(1.0) / safe_dir;

    let entry = (chunk_min - pos_ray_origin) * pos_inv_dir;
    let exit = (chunk_max - pos_ray_origin) * pos_inv_dir;

    let t_min = max(entry.x, max(entry.y, entry.z));
    let t_max = min(exit.x, min(exit.y, exit.z));

    if (t_min >= t_max || t_max < 0.0) { return false; }

    var stack: array<u32, 5>;

    var stack_subs: u32 = 0u;
    var sp: i32 = 0;

    let root_node_data = world_data[chunk_offset];
    
    var current_center = (chunk_min + chunk_max) * 0.5;
    var current_half_size = (chunk_max.x - chunk_min.x) * 0.5; 

    let mid = (entry + exit) * 0.5;
    let root_sub_voxel = get_first_child_intersect(t_min, entry, mid);
    
    stack_subs = root_sub_voxel;
    stack[0] = root_node_data;
    
    while (sp >= 0) {
        let shift = u32(sp) * 6u;

        // Read current frame
        var current_node = stack[sp];
        var raw_sub = (stack_subs >> shift) & 0x3Fu;
        
        let visited = (raw_sub & 16u) != 0u;
        let actual_sub = raw_sub & 15u; 

        // ASCEND
        if (actual_sub > 7u) {
            sp--; 
            if (sp >= 0) {
                let parent_shift = u32(sp) * 6u;
                let parent_sub = (stack_subs >> parent_shift) & 15u;

                current_center.x -= select(-current_half_size, current_half_size, (parent_sub & 1u) != 0u);
                current_center.y -= select(-current_half_size, current_half_size, (parent_sub & 2u) != 0u);
                current_center.z -= select(-current_half_size, current_half_size, (parent_sub & 4u) != 0u);
                current_half_size *= 2.0;
            }
            continue;
        }

        let voxel_min = current_center - vec3<f32>(current_half_size);
        let voxel_max = current_center + vec3<f32>(current_half_size);
        let f_entry = (voxel_min - pos_ray_origin) * pos_inv_dir;
        let f_exit = (voxel_max - pos_ray_origin) * pos_inv_dir;
        let f_mid = (f_entry + f_exit) * 0.5;

        let true_sub_voxel = actual_sub ^ direction_mask;
        let child_exists = has_child(current_node, true_sub_voxel);
        
        // DECEND
        if (!visited && child_exists) {
            let pointer = get_ending(current_node);
            let child_index = pointer + child_pop_count(current_node, true_sub_voxel);
            let node_at_index = world_data[chunk_offset + child_index];

            if (is_leaf(current_node, true_sub_voxel)) {
                 return true;
            }

            stack_subs |= (16u << shift);
            
            current_half_size *= 0.5;
            current_center.x += select(-current_half_size, current_half_size, (actual_sub & 1u) != 0u);
            current_center.y += select(-current_half_size, current_half_size, (actual_sub & 2u) != 0u);
            current_center.z += select(-current_half_size, current_half_size, (actual_sub & 4u) != 0u);

            let child_voxel_min = current_center - vec3<f32>(current_half_size);
            let child_voxel_max = current_center + vec3<f32>(current_half_size);
            let sub_entry = (child_voxel_min - pos_ray_origin) * pos_inv_dir;
            let sub_exit = (child_voxel_max - pos_ray_origin) * pos_inv_dir;
            
            let child_t_min = max(sub_entry.x, max(sub_entry.y, sub_entry.z));
            let child_mid = (sub_entry + sub_exit) * 0.5;

            sp++;
            let new_shift = u32(sp) * 6u;
            let new_sub = get_first_child_intersect(child_t_min, sub_entry, child_mid);

            stack[sp] = node_at_index;
            stack_subs = (stack_subs & ~(0x3Fu << new_shift)) | (new_sub << new_shift);

            continue;
        }

        let node_exit = vec3<f32>(
            select(f_mid.x, f_exit.x, (actual_sub & 1u) != 0u),
            select(f_mid.y, f_exit.y, (actual_sub & 2u) != 0u),
            select(f_mid.z, f_exit.z, (actual_sub & 4u) != 0u)
        );

        let next_sub = get_next_sub_voxel(actual_sub, vec_exit_plane(node_exit));
        stack_subs = (stack_subs & ~(0x3Fu << shift)) | (next_sub << shift);

    }
    
    return false;
}

// ==========================================
// 5. MAKRO-TRAVERSERING (cast_ray octree.rs)
// ==========================================

fn expand_bits(v: u32) -> u32 {
    var x = v & 0x000003FFu; 
    x = (x | (x << 16u)) & 0x030000FFu;
    x = (x | (x <<  8u)) & 0x0300F00Fu;
    x = (x | (x <<  4u)) & 0x030C30C3u;
    x = (x | (x <<  2u)) & 0x09249249u;
    return x;
}

fn get_chunk_root_pointer(chunk_pos: vec3<i32>, offset: i32, grid_size: i32) -> u32 {
    let gx = chunk_pos.x + offset;
    let gy = chunk_pos.y + offset;
    let gz = chunk_pos.z + offset;
    
    if (gx >= 0 && gx < grid_size && gy >= 0 && gy < grid_size && gz >= 0 && gz < grid_size) {
        
        let morton_x = expand_bits(u32(gx));
        let morton_y = expand_bits(u32(gy));
        let morton_z = expand_bits(u32(gz));
        
        let grid_index = morton_x | (morton_y << 1u) | (morton_z << 2u);
        
        return world_data[grid_index];
    }
    
    return 0xFFFFFFFFu;
}

fn cast_ray(origin: vec3<f32>, direction: vec3<f32>, limit: u32, out_payload: ptr<function, u32>, out_pos: ptr<function, vec3<f32>>, out_normal: ptr<function, vec3<f32>> , offset: i32, render_diameter: i32) -> bool {
    let chunk_size = 32.0;
    var chunk_pos = vec3<i32>(floor(origin / chunk_size));
    
    let safe_dir = vec3<f32>(
        select(direction.x, select(-0.0000001, 0.0000001, direction.x >= 0.0), abs(direction.x) < 0.0000001),
        select(direction.y, select(-0.0000001, 0.0000001, direction.y >= 0.0), abs(direction.y) < 0.0000001),
        select(direction.z, select(-0.0000001, 0.0000001, direction.z >= 0.0), abs(direction.z) < 0.0000001)
    );
    var inv_dir = 1.0 / safe_dir;

    var step = vec3<i32>(0);
    var t_max = vec3<f32>(0.0);
    var t_delta = vec3<f32>(0.0);
    
    //Divergence here was faster as the warp is almost always identical, maybe some smart calculation around the screen center can be done?
    // X
    t_delta.x = abs(chunk_size * inv_dir.x);
    if (direction.x > 0.0) {
        step.x = 1;
        t_max.x = ((f32(chunk_pos.x + 1) * chunk_size) - origin.x) * inv_dir.x;
    } else {
        step.x = -1;
        t_max.x = (origin.x - (f32(chunk_pos.x) * chunk_size)) * -inv_dir.x;
    }
    // Y
    t_delta.y = abs(chunk_size * inv_dir.y);
    if (direction.y > 0.0) {
        step.y = 1;
        t_max.y = ((f32(chunk_pos.y + 1) * chunk_size) - origin.y) * inv_dir.y;
    } else {
        step.y = -1;
        t_max.y = (origin.y - (f32(chunk_pos.y) * chunk_size)) * -inv_dir.y;
    }
    // Z
    t_delta.z = abs(chunk_size * inv_dir.z);
    if (direction.z > 0.0) {
        step.z = 1;
        t_max.z = ((f32(chunk_pos.z + 1) * chunk_size) - origin.z) * inv_dir.z;
    } else {
        step.z = -1;
        t_max.z = (origin.z - (f32(chunk_pos.z) * chunk_size)) * -inv_dir.z;
    }

    // DDA LOOPEN
    for (var i = 0u; i < limit; i++) {
        let chunk_root_ptr = get_chunk_root_pointer(chunk_pos, offset, render_diameter);
        
        if (chunk_root_ptr != 0xFFFFFFFFu) {
            let chunk_min = vec3<f32>(chunk_pos) * chunk_size;
            let chunk_max = chunk_min + vec3<f32>(chunk_size);
            
            // Dyk in i Micro SVO-traverseringen
            if (find_intersection(origin, direction, chunk_min, chunk_max, chunk_root_ptr, out_payload, out_pos, out_normal)) {
                return true;
            }
        }

        // Stega DDA
        if (t_max.x < t_max.y) {
            if (t_max.x < t_max.z) { chunk_pos.x += step.x; t_max.x += t_delta.x; }
            else { chunk_pos.z += step.z; t_max.z += t_delta.z; }
        } else {
            if (t_max.y < t_max.z) { chunk_pos.y += step.y; t_max.y += t_delta.y; }
            else { chunk_pos.z += step.z; t_max.z += t_delta.z; }
        }
    }
    return false;
}

fn cast_ray_anyhit(origin: vec3<f32>, direction: vec3<f32>, limit: u32, offset: i32, render_diameter: i32) -> bool {
    let chunk_size = 32.0;
    var chunk_pos = vec3<i32>(floor(origin / chunk_size));
    
    let safe_dir = vec3<f32>(
        select(direction.x, select(-0.0000001, 0.0000001, direction.x >= 0.0), abs(direction.x) < 0.0000001),
        select(direction.y, select(-0.0000001, 0.0000001, direction.y >= 0.0), abs(direction.y) < 0.0000001),
        select(direction.z, select(-0.0000001, 0.0000001, direction.z >= 0.0), abs(direction.z) < 0.0000001)
    );
    var inv_dir = 1.0 / safe_dir;

    var step = vec3<i32>(0); var t_max = vec3<f32>(0.0); var t_delta = vec3<f32>(0.0);

    t_delta.x = abs(chunk_size * inv_dir.x);
    if (direction.x > 0.0) {
        step.x = 1;
        t_max.x = ((f32(chunk_pos.x + 1) * chunk_size) - origin.x) * inv_dir.x;
    } else {
        step.x = -1;
        t_max.x = (origin.x - (f32(chunk_pos.x) * chunk_size)) * -inv_dir.x;
    }
    // Y
    t_delta.y = abs(chunk_size * inv_dir.y);
    if (direction.y > 0.0) {
        step.y = 1;
        t_max.y = ((f32(chunk_pos.y + 1) * chunk_size) - origin.y) * inv_dir.y;
    } else {
        step.y = -1;
        t_max.y = (origin.y - (f32(chunk_pos.y) * chunk_size)) * -inv_dir.y;
    }
    // Z
    t_delta.z = abs(chunk_size * inv_dir.z);
    if (direction.z > 0.0) {
        step.z = 1;
        t_max.z = ((f32(chunk_pos.z + 1) * chunk_size) - origin.z) * inv_dir.z;
    } else {
        step.z = -1;
        t_max.z = (origin.z - (f32(chunk_pos.z) * chunk_size)) * -inv_dir.z;
    }
    
    for (var i = 0u; i < limit; i++) {
        let chunk_root_ptr = get_chunk_root_pointer(chunk_pos, offset, render_diameter);
        if (chunk_root_ptr != 0xFFFFFFFFu) {
            let chunk_min = vec3<f32>(chunk_pos) * chunk_size;
            let chunk_max = chunk_min + vec3<f32>(chunk_size);
            
            if (find_intersection_anyhit(origin, direction, chunk_min, chunk_max, chunk_root_ptr)) {
                return true;
            }
        }
        // Stega DDA
        if (t_max.x < t_max.y) {
            if (t_max.x < t_max.z) { chunk_pos.x += step.x; t_max.x += t_delta.x; }
            else { chunk_pos.z += step.z; t_max.z += t_delta.z; }
        } else {
            if (t_max.y < t_max.z) { chunk_pos.y += step.y; t_max.y += t_delta.y; }
            else { chunk_pos.z += step.z; t_max.z += t_delta.z; }
        }
    }
    return false;
}

// ==========================================
// 6. WAVEFRONT KERNELS (Split passes)
// ==========================================

// ------------------------------------------
// Pass 1: Camera Rays
// ------------------------------------------

@compute @workgroup_size(8, 8, 1)
fn ray_gen_pass(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(screen_texture);

    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y) {
        return;
    }

    let x = f32(global_id.x);
    let y = f32(global_id.y);
    let ray_dir = camera.top_left + (camera.delta_x * x) + (camera.delta_y * y);

    let render_radius = i32(camera.render_distance);
    let render_diameter = render_radius * 2;
    let ray_dda_limit = u32(render_diameter);

    var payload: u32 = 0u;
    var hit_pos: vec3<f32> = vec3<f32>(0.0);
    var hit_normal: vec3<f32> = vec3<f32>(0.0);

    let hit = cast_ray(camera.position, normalize(ray_dir), ray_dda_limit, &payload, &hit_pos, &hit_normal, render_radius, render_diameter);

    let index = global_id.y * dimensions.x + global_id.x;
    hit_buffer[index] = HitData(hit_pos, u32(hit), hit_normal, payload);
}

// ------------------------------------------
// Pass 2: Shading
// ------------------------------------------

@compute @workgroup_size(8, 8, 1)
fn shading_pass(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(screen_texture);
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y) { return; }

    let index = global_id.y * dimensions.x + global_id.x;
    let hit_info = hit_buffer[index];

    var final_color = lighting.sky_color; 

    let shadow_dir = normalize(lighting.sun_direction);
    
    if (hit_info.did_hit == 1u) {
        let max_index = arrayLength(&colour_lut) - 1u; 
        var base_color = colour_lut[min(hit_info.payload, max_index)]; //fine
        
        let side_multiplier = get_face_multiplier(hit_info.hit_normal);
        let dot_light = dot(hit_info.hit_normal, shadow_dir);
        
        if (dot_light <= 0.0) {
            final_color = base_color * lighting.ambient_strength; //THESE HAVE THE WRONG SIDE
        } else {
            // Push ray slightly off surface
            let shadow_origin = hit_info.hit_pos + (hit_info.hit_normal * 0.001);
            let render_radius = i32(camera.render_distance);
            
            // Cast Shadow Ray
            let is_occluded = cast_ray_anyhit(shadow_origin, shadow_dir, u32(render_radius * 2), render_radius, render_radius * 2);
            
            if (is_occluded) {
                final_color = base_color * lighting.ambient_strength; 
            } else {
                final_color = base_color * side_multiplier; 
            }
        }
    } else {

            let x = f32(global_id.x);
            let y = f32(global_id.y);
            let ray_dir = normalize(camera.top_left + (camera.delta_x * x) + (camera.delta_y * y));

            let sky_multiplier = dot(ray_dir, shadow_dir) * 0.5 + 0.5;
            final_color = vec4<f32>(lighting.sky_color.rgb * sky_multiplier, lighting.sky_color.a);

    }

    textureStore(screen_texture, global_id.xy, final_color);
}

