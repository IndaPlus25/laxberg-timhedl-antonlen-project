use rand::RngExt; 
use noise::{Fbm, MultiFractal, NoiseFn, Perlin, Simplex};
use crate::vecmath::V3i;

enum Biome {
    Plains,
    Mountains,
}

struct BiomeNoise {
    plains: Fbm<Perlin>,
    mountains: Fbm<Perlin>
}

pub struct BlockColors {
    pub grass: u32,
    pub stone: u32,
    pub water: u32,
}

impl BiomeNoise {
    fn new(seed: u32) -> Self {
        Self {
            plains: Fbm::<Perlin>::new(seed)
                .set_octaves(2)
                .set_frequency(0.01)
                .set_persistence(0.3),
            mountains: Fbm::<Perlin>::new(seed)
                .set_octaves(6)
                .set_frequency(0.005)
                .set_persistence(0.6),
        }
    }
}

fn biome_height_limit(biome: Biome, pos: [f64; 2], functions: &BiomeNoise) -> f64{
    let (noise_value, base_height, height_diff) = match biome {
        Biome::Plains => {
            (functions.plains.get(pos), 40.0, 10.0)
        },
        Biome::Mountains => {
            (functions.mountains.get(pos), 64.0, 90.0)
        },
    };

    let normalised = ((noise_value + 1.0) / 2.0).clamp(0.0, 1.0);
    base_height + (normalised * height_diff)
}

pub fn generate_single_chunk(colors: &BlockColors, seed: u32, chunk_coord: &V3i) -> Vec<u32> {
    let mut flat_data = vec![0; 32768];
    let functions = BiomeNoise::new(seed);
    let simplex = Simplex::new(seed);
    let biome_closeness = 0.001;

    for dx in 0..32 {
        for dz in 0..32 {

            let global_x = chunk_coord.x * 32 + dx;
            let global_z = chunk_coord.z * 32 + dz;

            let biome_noise = simplex.get([biome_closeness * global_x as f64, biome_closeness * global_z as f64]);
            let plains_limit = biome_height_limit(Biome::Plains, [global_x as f64, global_z as f64], &functions);
            let mountains_limit = biome_height_limit(Biome::Mountains, [global_x as f64, global_z as f64], &functions);

            let global_y_limit = ((1.0 - biome_noise) * plains_limit + (biome_noise - 0.1).clamp(0., 1.) * (biome_noise + 1.) * mountains_limit) as i32; 

            for dy in 0..32 {
                let index = dx + (dy * 32) + (dz * 32 * 32);

                let global_y = chunk_coord.y * 32 + dy;
                if 0 < global_y && global_y <= global_y_limit {
                    let color = if biome_noise > 0.2 {
                        colors.stone
                    } else {
                        colors.grass
                    };
                    flat_data[index as usize] = color;
                }

                if global_y <= 0 {
                    flat_data[index as usize] = colors.water;
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
