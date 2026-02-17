#import bevy_sprite::mesh2d_view_bindings::globals
#import bevy_sprite::bevy_sprite::mesh2d_view_bindings::view
#import bevy_sprite::mesh2d_functions as mesh_functions

const GROUND_PHASE_DURATION: f32 = 0.2;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) lookup_index: u32,
    @location(4) frame_count: u32,
    @location(5) patterns: vec4<u32>,
};

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) lookup_index: u32,
    @location(4) frame_count: u32,
    @location(5) patterns: vec4<u32>,
};

struct SpriteUv {
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var atlas: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var atlas_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> time_offset: f32;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> atlas_grid: vec2<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<storage, read> lookup_map: array<u32>;

@vertex
fn vertex(
    vertex: Vertex
) -> VertexOutput {
    var out: VertexOutput;
    
    out.uv = vertex.uv;
    
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0)
    );
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
    
    out.lookup_index = vertex.lookup_index;
    out.frame_count = vertex.frame_count;
    out.patterns = vertex.patterns;
    
    return out;
}

fn atlas_uv(base_uv: vec2<f32>, sprite_index: u32) -> vec2<f32> {
    let cols = u32(atlas_grid.x);
    let rows = u32(atlas_grid.y);

    let tile_size = vec2<f32>(
        1.0 / atlas_grid.x,
        1.0 / atlas_grid.y
    );

    let col = sprite_index % cols;
    let row = sprite_index / cols;

    let offset = vec2<f32>(
        f32(col) * tile_size.x,
        f32(row) * tile_size.y
    );

    let inset = (tile_size / 32.0) * 0.5;
    let usable = tile_size - inset * 2.0;

    return offset + inset + base_uv * usable;
}

fn compute_index(phase: u32, patterns: vec4<u32>) -> u32 {
    return (
        (phase * patterns[1] + patterns[3])
        * patterns[0] + patterns[2]
    );
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let time = globals.time;
    let t = time - time_offset;
    let p = floor(t / GROUND_PHASE_DURATION);
    let phase = u32(p) % in.frame_count;

    let index = compute_index(phase, in.patterns);
    let sprite_index = lookup_map[in.lookup_index + index];
    let uv = atlas_uv(in.uv, sprite_index);

    return textureSample(atlas, atlas_sampler, uv);
}