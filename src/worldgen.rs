use rand::RngExt; 


pub fn generate_single_chunk(color: u32) -> Vec<u32> {
    let mut flat_data = vec![0; 32768];

    for dx in 0..32 {
        for dz in 0..32 {
            for dy in 0..32 {
                let index = dx + (dy * 32) + (dz * 32 * 32);
                if dy < 16 {
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
