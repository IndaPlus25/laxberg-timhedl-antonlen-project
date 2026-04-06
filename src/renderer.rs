use crate::vecmath::*;
use crate::octree::*;

pub struct Player {
    pub position: V3,
    pub direction: (f32, f32),
}

pub fn default_color(buffer: &mut [u32], width: u32, height: u32) {
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

pub fn raycaster(buffer: &mut [u32], width: u32, height: u32, fov: f32, player: Player, chunk: &Chunk) {

    let aspect_ratio = width as f32/height as f32;

    let plane_width = 2.0 * (fov / 2.0).tan();
    let plane_height = plane_width / aspect_ratio;

    let global_up = V3 { x: 0.0, y: 0.0, z: 1.0 };

    let forward_vec = V3 {
        x: (player.direction.1).cos() * (player.direction.0).sin(),
        y: (player.direction.1).sin(),
        z: (player.direction.1).cos() * (player.direction.0).cos()
    };

    let right_vec = vec_normalize(&vec_crossp(&global_up, &forward_vec));

    let up_vec = vec_normalize(&vec_crossp(&forward_vec, &right_vec));

    let top_left_vec = vec_add(
        &vec_sub(&forward_vec, &vec_mult_scal(&right_vec, plane_width / 2.0)), 
        &vec_mult_scal(&up_vec, plane_height / 2.0)
    );

    let step_x_size = plane_width / width as f32;
    let step_y_size = plane_height / height as f32;
    
    let delta_x = vec_mult_scal(&right_vec, step_x_size);
    let delta_y = vec_mult_scal(&up_vec, -step_y_size);

    for (index, pixel) in buffer.iter_mut().enumerate() {
        let x = (index % width as usize) as f32;
        let y = (index / width as usize) as f32;

        let x_offset = vec_mult_scal(&delta_x, x);
        let y_offset = vec_mult_scal(&delta_y, y);

        let big_ray_dir = vec_add(&top_left_vec, &vec_add(&x_offset, &y_offset));

        let ray_dir = vec_normalize(&big_ray_dir);

        let ray = Ray { origin: player.position, direction: ray_dir };

        let intersection = find_intersection(&ray, chunk, chunk.data[0]);

        if let Some(hit) = intersection {
            let b = get_ending(hit.voxel_data);
            *pixel = match b {
                1 => 0xFF0000, // Red
                2 => 0x00FF00, // Green
                3 => 0x0000FF, // Blue
                _ => 0xFFFFFF, // White
            };
        } else {
            *pixel = 0; // Your gray/blue background
        }
    }
}
