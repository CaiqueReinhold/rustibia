#import bevy_sprite::mesh2d_view_bindings::globals
#import bevy_sprite::mesh2d_functions as mesh_functions

struct ItemInstance {
    phase_count: u32,
    phase_duration: f32,
    time_offset: f32,
    lookup_offset: u32,
    pattern_x: u32,
    pattern_y: u32,
    pattern_z: u32,
    value_x: u32,
    value_y: u32,
    value_z: u32,
    bounding_square: f32,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var atlas_tex: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var atlas_smp: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> time_offset: f32;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> atlas_grid: vec2<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var<storage, read> sprite_lookup: array<u32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var<storage, read> instances: array<ItemInstance>;

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

fn calculate_world_pos_with_bbox_crop(
    position: vec3<f32>,
    square: f32,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>,
    instance_index: u32
) -> vec4<f32> {
    let local01 = (position.xy + vec2<f32>(32.0)) / 64.0;
    let logical_pos = (local01 - vec2<f32>(0.5)) * square;
    let bbox_center = vec2<f32>(
        bbox_min.x + bbox_size.x * 0.5 - square * 0.5,
        square * 0.5 - (bbox_min.y + bbox_size.y * 0.5)
    );
    let cropped_local =
        logical_pos * (bbox_size / square) + bbox_center;

    var world_from_local = mesh_functions::get_world_from_local(instance_index);
    return mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(cropped_local, position.z, 1.0)
    );
}

fn adjust_uv_to_bbox(
    uv: vec2<f32>,
    square: f32,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>
) -> vec2<f32> {
    let bbox_min_n = bbox_min / square;
    let bbox_size_n = bbox_size / square;
    return bbox_min_n + uv * bbox_size_n;
}

fn get_animation_phase(instance: ItemInstance) -> u32 {
    if (instance.phase_duration == 0u) {
        return 0;
    }
    let t = globals.time - instance.time_offset;
    let p = floor(t / instance.phase_duration);
    let phase = u32(p) % instance.phase_count;
    return phase;
}

fn atlas_uv(base_uv: vec2<f32>, index: u32) -> vec2<f32> {
    let cols = u32(params.atlas_grid.x);
    let rows = u32(params.atlas_grid.y);

    let tile_size = vec2<f32>(
        1.0 / params.atlas_grid.x,
        1.0 / params.atlas_grid.y
    );

    let col = index % cols;
    let row = index / cols;

    let offset = vec2<f32>(
        f32(col) * tile_size.x,
        f32(row) * tile_size.y
    );

    return offset + base_uv * tile_size;
}

fn compute_index(
    phase: u32,
    value_x: u32,
    value_y: u32,
    value_z: u32,
    pattern_x: u32,
    pattern_y: u32,
    pattern_z: u32,
) -> u32 {
    return (
        (
            (phase * pattern_z + value_z)
            * pattern_y + value_y
        )
        * pattern_x + value_x
    );
}

// @vertex
// fn vertex(vertex: Vertex) -> VertexOutput {
//     var out: VertexOutput;

//     let inst_index = mesh_functions::get_tag(vertex.instance_index);
//     let inst = instances[inst_index];
//     let square = inst.bounding_square;
//     let bbox_min = inst.bbox_min;
//     let bbox_size = inst.bbox_size;

//     out.world_position = calculate_world_pos_with_bbox_crop(
//         vertex.position,
//         square,
//         bbox_min,
//         bbox_size,
//         vertex.instance_index
//     );
//     out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
//     let base_uv = adjust_uv_to_bbox(vertex.uv, square, bbox_min, bbox_size);
//     let phase = get_animation_phase(inst);
//     let lookup_index = compute_index(
//         phase,
//         inst.value_x,
//         inst.value_y,
//         inst.value_z,
//         inst.pattern_x,
//         inst.pattern_y,
//         inst.pattern_z,
//     ) + inst.lookup_offset;
//     let atlas_index = sprite_lookup[lookup_index];
//     out.uv = atlas_uv(base_uv, 0);
    
//     return out;
// }

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
    
    return out;
}



@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0);
    return textureSample(atlas_tex, atlas_smp, in.uv);
}