#import bevy_sprite::mesh2d_functions as mesh_functions

struct ItemInstance {
    sprite_id: u32,
    _pad: u32,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var atlas_tex: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var atlas_smp: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> atlas_grid: vec2<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<storage, read> instances: array<ItemInstance>;

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var<uniform> mesh_size: vec2<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(2) uv: vec2<f32>
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>
}

fn calculate_world_pos(
    position: vec3<f32>,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>,
    instance_index: u32
) -> vec4<f32> {
    let local01 = position.xy / mesh_size;
    var scaled = local01 * bbox_size;
    let bbox_center = (bbox_min * 2.0 + bbox_size - mesh_size) * vec2<f32>(0.5, -0.5);
    let final_local = scaled + bbox_center;
    var world_from_local = mesh_functions::get_world_from_local(instance_index);
    return mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(final_local, position.z, 1.0)
    );
}

fn adjust_uv_to_bbox(uv: vec2<f32>, bbox_min: vec2<f32>, bbox_size: vec2<f32>) -> vec2<f32> {
    let bbox_min_n = bbox_min / mesh_size;
    let bbox_size_n = bbox_size / mesh_size;
    return bbox_min_n + uv * bbox_size_n;
}

fn atlas_uv(base_uv: vec2<f32>, index: u32) -> vec2<f32> {
    let tile_size = vec2<f32>(1.0 / atlas_grid.x, 1.0 / atlas_grid.y);
    let col = index % u32(atlas_grid.x);
    let row = index / u32(atlas_grid.x);
    let offset = vec2<f32>(f32(col) * tile_size.x, f32(row) * tile_size.y);
    let inset = (tile_size / mesh_size) * 0.2;
    let usable = tile_size - inset * 2.0;
    return offset + inset + base_uv * usable;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let inst_index = mesh_functions::get_tag(vertex.instance_index);
    let inst = instances[inst_index];
    out.world_position = calculate_world_pos(
        vertex.position, inst.bbox_min, inst.bbox_size, vertex.instance_index
    );
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
    let base_uv = adjust_uv_to_bbox(vertex.uv, inst.bbox_min, inst.bbox_size);
    out.uv = atlas_uv(base_uv, inst.sprite_id);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(atlas_tex, atlas_smp, in.uv);
}
