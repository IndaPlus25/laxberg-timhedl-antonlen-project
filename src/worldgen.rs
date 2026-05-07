use rand::RngExt; 
use noise::{NoiseFn, Perlin};


pub fn generate_single_chunk(color: u32, seed: u32) -> Vec<u32> {
    let mut flat_data = vec![0; 32768];
    let perlin = Perlin::new(seed);
    let scale = 0.01;

    for dx in 0..32 {
        for dz in 0..32 {
            let noise_value = (perlin.get([scale * dx as f64 , scale * dz as f64]) + 1.0) / 2.0;
            let limit = (16.0 + (noise_value * 16.0)) as u32;
            println!("{}", noise_value);

            for dy in 0..32 {
                let index = dx + (dy * 32) + (dz * 32 * 32);
                if (dy as u32) < limit{
                    flat_data[index] = color;
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
