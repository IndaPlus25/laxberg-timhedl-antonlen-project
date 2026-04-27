// Plats 0: Vår färg (skickad från CPU)
@group(0) @binding(0) var<storage, read> input_color: vec4<f32>;

// Plats 1: Vår skärm (där vi ska rita)
@group(0) @binding(1) var screen_texture: texture_storage_2d<bgra8unorm, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(screen_texture);
    
    // Om tråden hamnar utanför skärmens dimensioner, avbryt!
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y) {
        return;
    }

    // Läs färgen
    let color_to_draw = input_color;

    // Rita färgen på skärmen!
    textureStore(screen_texture, global_id.xy, color_to_draw);
}
