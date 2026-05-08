use rand::RngExt; 
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use crate::vecmath::V3i;


pub fn generate_single_chunk(color: u32, seed: u32, chunk_coord: &V3i) -> Vec<u32> {
    let mut flat_data = vec![0; 32768];
    let fbm = Fbm::<Perlin>::new(seed)
        .set_octaves(6)
        .set_frequency(0.003)
        .set_persistence(0.6);

    for dx in 0..32 {
        for dz in 0..32 {

            let global_x = chunk_coord.x * 32 + dx;
            let global_z = chunk_coord.z * 32 + dz;

            let noise_value = fbm.get([global_x as f64, global_z as f64]);
            let normalized_noise = ((noise_value + 1.0) / 2.0).clamp(0.0, 1.0);

            let global_y_limit = (64.0 + (normalized_noise * 90.0)) as i32;

            for dy in 0..32 {
                let index = dx + (dy * 32) + (dz * 32 * 32);

                let global_y = chunk_coord.y * 32 + dy;
                if global_y < global_y_limit {
                    flat_data[index as usize] = color;
                }
            }
        }
    }
    flat_data
}

pub fn generate_random_world(width: usize, height: usize, depth: usize, density: f64, max_material: u32) -> Vec<Vec<Vec<u32>>> {

    let mut world = vec![vec![vec![0; depth]; height]; width];
    

    let mut rng = rand::rng();

    for x in 0..width {
        for y in 0..height {
            for z in 0..depth {
                // Determine if a block should spawn here based on the density
                if rng.random_bool(density) {
                    // Pick a random material ID between 1 and 8
                    world[x][y][z] = rng.random_range(1..=max_material);
                }
            }
        }
    }

    world
}
