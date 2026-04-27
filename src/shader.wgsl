// ==========================================
// 1. DATASTRUKTURER & BIND GROUPS
// ==========================================

struct Camera {
    position: vec3<f32>,
    direction: vec2<f32>, // x = yaw, y = pitch
    fov: f32,
    aspect_ratio: f32,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var screen_texture: texture_storage_2d<bgra8unorm, write>;

// Detta är hela din värld! Alla voxels från alla chunks ligger i en lång lista
@group(0) @binding(2) var<storage, read> world_data: array<u32>; 

// ==========================================
// 2. HJÄLPFUNKTIONER (Från vecmath.rs)
// ==========================================

fn vec_entry_plane(v: vec3<f32>) -> u32 {
    if (v.x > v.y && v.x > v.z) { return 0u; } // YZ plane
    else if (v.y > v.x && v.y > v.z) { return 1u; } // XZ plane
    else { return 2u; } // XY plane
}

fn vec_exit_plane(v: vec3<f32>) -> u32 {
    if (v.x < v.y && v.x < v.z) { return 0u; } // YZ
    else if (v.y < v.x && v.y < v.z) { return 1u; } // XZ
    else { return 2u; } // XY
}

// ==========================================
// 3. BITWISE FUNKTIONER (Från octree.rs)
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
// 4. MIKRO-TRAVERSERING (Ersätter proc_subtree via Stack)
// ==========================================

// Eftersom vi inte kan använda rekursion, sparar vi "vart vi är" i denna struct
struct StackFrame {
    node_data: u32,
    sub_voxel: u32,
    entry: vec3<f32>,
    exit: vec3<f32>,
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

// Detta är Revelles et al. transition tabell exakt som i din Rust-kod
fn get_next_sub_voxel(current: u32, exit_plane: u32) -> u32 {
    if (current == 0u) {
        if (exit_plane == 0u) { return 1u; } else if (exit_plane == 1u) { return 2u; } else { return 4u; }
    } else if (current == 1u) {
        if (exit_plane == 0u) { return 8u; } else if (exit_plane == 1u) { return 3u; } else { return 5u; }
    } else if (current == 2u) {
        if (exit_plane == 0u) { return 3u; } else if (exit_plane == 1u) { return 8u; } else { return 6u; }
    } else if (current == 3u) {
        if (exit_plane == 0u) { return 8u; } else if (exit_plane == 1u) { return 8u; } else { return 7u; }
    } else if (current == 4u) {
        if (exit_plane == 0u) { return 5u; } else if (exit_plane == 1u) { return 6u; } else { return 8u; }
    } else if (current == 5u) {
        if (exit_plane == 0u) { return 8u; } else if (exit_plane == 1u) { return 7u; } else { return 8u; }
    } else if (current == 6u) {
        if (exit_plane == 0u) { return 7u; } else if (exit_plane == 1u) { return 8u; } else { return 8u; }
    }
    return 8u; // 8 betyder "Lämna noden" (return None i Rust)
}

fn find_intersection(ray_origin: vec3<f32>, ray_dir: vec3<f32>, chunk_min: vec3<f32>, chunk_max: vec3<f32>, root_pointer: u32, out_payload: ptr<function, u32>) -> bool {
    var direction_mask: u32 = 0u;
    var pos_ray_dir = ray_dir;
    var pos_ray_origin = ray_origin;

    // Spegla ray för SVO traverseringen (exakt som i din kod)
    if (pos_ray_dir.x < 0.0) { direction_mask |= 1u; pos_ray_dir.x = -pos_ray_dir.x; pos_ray_origin.x = chunk_max.x - (ray_origin.x - chunk_min.x); }
    if (pos_ray_dir.y < 0.0) { direction_mask |= 2u; pos_ray_dir.y = -pos_ray_dir.y; pos_ray_origin.y = chunk_max.y - (ray_origin.y - chunk_min.y); }
    if (pos_ray_dir.z < 0.0) { direction_mask |= 4u; pos_ray_dir.z = -pos_ray_dir.z; pos_ray_origin.z = chunk_max.z - (ray_origin.z - chunk_min.z); }

    let entry = (chunk_min - pos_ray_origin) / pos_ray_dir;
    let exit = (chunk_max - pos_ray_origin) / pos_ray_dir;

    let t_min = max(entry.x, max(entry.y, entry.z));
    let t_max = min(exit.x, min(exit.y, exit.z));

    if (t_min >= t_max || t_max < 0.0) { return false; }

    // INITIALISERA STACKEN (Max SVO djup för 32^3 är 5)
    var stack: array<StackFrame, 6>;
    var sp: i32 = 0; // Stack pointer
    
    let mid = (entry + exit) * 0.5;
    let root_sub_voxel = get_first_child_intersect(t_min, entry, mid);
    stack[0] = StackFrame(root_pointer, root_sub_voxel, entry, exit);

    // ITERATIV PROC_SUBTREE LÖÖP
    while (sp >= 0) {
        let frame = stack[sp];
        let current_node = frame.node_data;
        let current_sub = frame.sub_voxel;
        let f_entry = frame.entry;
        let f_exit = frame.exit;
        let f_mid = (f_entry + f_exit) * 0.5;

        // Om sub_voxel är 8 eller högre betyder det att vi klivit ur denna voxel. (None)
        if (current_sub > 7u) {
            sp--; // Pop (Gå upp en nivå)
            continue;
        }

        let true_sub_voxel = current_sub ^ direction_mask;

        if (has_child(current_node, true_sub_voxel)) {
            let pointer = get_ending(current_node);
            let child_index = pointer + child_pop_count(current_node, true_sub_voxel);
            
            // HÄMTA NODEN FRÅN VRAM
            let node_at_index = world_data[child_index];

            if (is_leaf(current_node, true_sub_voxel)) {
                *out_payload = get_ending(node_at_index);
                return true;
            } else {
                // Bygg childs entry och exit boundaries
                let sub_entry = vec3<f32>(
                    select(f_entry.x, f_mid.x, (current_sub & 1u) != 0u),
                    select(f_entry.y, f_mid.y, (current_sub & 2u) != 0u),
                    select(f_entry.z, f_mid.z, (current_sub & 4u) != 0u)
                );
                let sub_exit = vec3<f32>(
                    select(f_mid.x, f_exit.x, (current_sub & 1u) != 0u),
                    select(f_mid.y, f_exit.y, (current_sub & 2u) != 0u),
                    select(f_mid.z, f_exit.z, (current_sub & 4u) != 0u)
                );

                // Räkna ut vad NÄSTA sub_voxel för vår nuvarande nivå ska bli när vi kommer tillbaka!
                let node_exit = vec3<f32>(
                    select(f_mid.x, f_exit.x, (current_sub & 1u) != 0u),
                    select(f_mid.y, f_exit.y, (current_sub & 2u) != 0u),
                    select(f_mid.z, f_exit.z, (current_sub & 4u) != 0u)
                );
                let exit_plane = vec_exit_plane(node_exit);
                
                // Uppdatera nuvarande frame innan vi går djupare
                stack[sp].sub_voxel = get_next_sub_voxel(current_sub, exit_plane);

                // PUSH: Dyk ner en nivå!
                sp++;
                let child_t_min = max(sub_entry.x, max(sub_entry.y, sub_entry.z));
                let child_mid = (sub_entry + sub_exit) * 0.5;
                stack[sp] = StackFrame(node_at_index, get_first_child_intersect(child_t_min, sub_entry, child_mid), sub_entry, sub_exit);
                continue;
            }
        }

        // Om inget barn fanns, stega till nästa sub_voxel direkt
        let node_exit = vec3<f32>(
            select(f_mid.x, f_exit.x, (current_sub & 1u) != 0u),
            select(f_mid.y, f_exit.y, (current_sub & 2u) != 0u),
            select(f_mid.z, f_exit.z, (current_sub & 4u) != 0u)
        );
        let exit_plane = vec_exit_plane(node_exit);
        stack[sp].sub_voxel = get_next_sub_voxel(current_sub, exit_plane);
    }

    return false;
}

// ==========================================
// 5. MAKRO-TRAVERSERING (Från cast_ray i octree.rs)
// ==========================================

// Dummy lookup funktion - I framtiden kommer du byta denna mot en koll i din Chunk Macro Grid
fn get_chunk_root_pointer(chunk_pos: vec3<i32>) -> u32 {
    // Returnerar offset till roten av chunk-trädet inuti `world_data`
    // Om chunk inte finns, returnera 0xFFFFFFFFu
    if (chunk_pos.x == 0 && chunk_pos.y == 0 && chunk_pos.z == 0) {
        return 0u; // Låtsas att Chunk (0,0,0) ligger på index 0
    }
    return 0xFFFFFFFFu;
}

fn cast_ray(origin: vec3<f32>, direction: vec3<f32>, limit: u32, out_payload: ptr<function, u32>) -> bool {
    let chunk_size = 32.0;
    var chunk_pos = vec3<i32>(floor(origin / chunk_size));
    
    // Förhindra division med noll via epsilon check, annars ger WGSL infinities
    var inv_dir = vec3<f32>(
        select(1.0 / direction.x, 9999999.0, abs(direction.x) < 0.000001),
        select(1.0 / direction.y, 9999999.0, abs(direction.y) < 0.000001),
        select(1.0 / direction.z, 9999999.0, abs(direction.z) < 0.000001)
    );

    var step = vec3<i32>(0);
    var t_max = vec3<f32>(0.0);
    var t_delta = vec3<f32>(0.0);

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

    // DDA LÖÖPEN
    for (var i = 0u; i < limit; i++) {
        let chunk_root_ptr = get_chunk_root_pointer(chunk_pos);
        
        if (chunk_root_ptr != 0xFFFFFFFFu) {
            let chunk_min = vec3<f32>(chunk_pos) * chunk_size;
            let chunk_max = chunk_min + vec3<f32>(chunk_size);
            
            // Dyk in i Micro SVO-traverseringen
            if (find_intersection(origin, direction, chunk_min, chunk_max, world_data[chunk_root_ptr], out_payload)) {
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
// 6. MAIN (Ersätter raycaster i renderer.rs)
// ==========================================

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(screen_texture);
    let width = f32(dimensions.x);
    let height = f32(dimensions.y);
    let x = f32(global_id.x);
    let y = f32(global_id.y);

    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y) {
        return;
    }

    let plane_width = 2.0 * tan(camera.fov / 2.0);
    let plane_height = plane_width / camera.aspect_ratio;
    let global_up = vec3<f32>(0.0, 1.0, 0.0);

    // Konvertera direction angles till en framåt-vektor
    let forward_vec = vec3<f32>(
        cos(camera.direction.y) * sin(camera.direction.x),
        sin(camera.direction.y),
        cos(camera.direction.y) * cos(camera.direction.x)
    );

    let right_vec = normalize(cross(global_up, forward_vec));
    let up_vec = normalize(cross(forward_vec, right_vec));

    let top_left_vec = forward_vec - (right_vec * (plane_width / 2.0)) + (up_vec * (plane_height / 2.0));

    let step_x_size = plane_width / width;
    let step_y_size = plane_height / height;
    
    let delta_x = right_vec * step_x_size;
    let delta_y = up_vec * -step_y_size;

    let ray_dir = normalize(top_left_vec + (delta_x * x) + (delta_y * y));

    // Kasta strålen!
    var payload: u32 = 0u;
    let hit = cast_ray(camera.position, ray_dir, 32u, &payload);

    var final_color = vec4<f32>(0.0, 0.0, 0.0, 1.0); // Svart bakgrund

    if (hit) {
        // Exakt samma färgsättning som du hade!
        if (payload == 1u) { final_color = vec4<f32>(1.0, 0.0, 0.0, 1.0); }       // Röd
        else if (payload == 2u) { final_color = vec4<f32>(0.0, 1.0, 0.0, 1.0); }  // Grön
        else if (payload == 3u) { final_color = vec4<f32>(0.0, 0.0, 1.0, 1.0); }  // Blå
        else if (payload == 4u) { final_color = vec4<f32>(1.0, 0.58, 0.0, 1.0); } // Orange
        else if (payload == 5u) { final_color = vec4<f32>(1.0, 0.83, 0.03, 1.0); }// Gul
        else { final_color = vec4<f32>(1.0, 1.0, 1.0, 1.0); }                     // Vit
    }

    textureStore(screen_texture, global_id.xy, final_color);
}
