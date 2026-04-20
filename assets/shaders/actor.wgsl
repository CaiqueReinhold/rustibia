#import bevy_sprite::mesh2d_functions as mesh_functions

struct ActorParams {
    atlas_grid: vec2<f32>,
}

struct ActorInstance {
    sprite_ids: array<u32, 6>,
    layer_count: u32,
    _pad: u32,
    color_head: vec4<f32>,
    color_body: vec4<f32>,
    color_legs: vec4<f32>,
    color_feet: vec4<f32>,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var atlas_tex: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var atlas_smp: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> params: ActorParams;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<storage, read> instances: array<ActorInstance>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) instance_index: u32,
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>
}

fn calculate_world_pos(
    position: vec3<f32>,
    mesh_size: f32,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>,
    instance_index: u32
) -> vec4<f32> {
    let local01 = position.xy / mesh_size;
    var scaled = local01 * bbox_size;
    let bbox_center = (bbox_min * 2.0 + bbox_size - mesh_size) * vec2<f32>(0.5, -0.5);
    let padding = vec2<f32>(7, -8);
    let final_local = scaled + bbox_center - padding;
    var world_from_local = mesh_functions::get_world_from_local(instance_index);
    return mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(final_local, position.z, 1.0)
    );
}

fn adjust_uv_to_bbox(
    uv: vec2<f32>,
    mesh_size: f32,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>
) -> vec2<f32> {
    let bbox_min_n = bbox_min / mesh_size;
    let bbox_size_n = bbox_size / mesh_size;
    return bbox_min_n + uv * bbox_size_n;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let inst_index = mesh_functions::get_tag(vertex.instance_index);
    let inst = instances[inst_index];
    out.world_position = calculate_world_pos(
        vertex.position, 64.0, inst.bbox_min, inst.bbox_size, vertex.instance_index
    );
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
    out.uv = adjust_uv_to_bbox(vertex.uv, 64.0, inst.bbox_min, inst.bbox_size);
    out.instance_index = inst_index;
    return out;
}

fn atlas_uv(base_uv: vec2<f32>, index: u32) -> vec2<f32> {
    let tile_size = vec2<f32>(1.0 / params.atlas_grid.x, 1.0 / params.atlas_grid.y);
    let col = index % u32(params.atlas_grid.x);
    let row = index / u32(params.atlas_grid.x);
    let offset = vec2<f32>(f32(col) * tile_size.x, f32(row) * tile_size.y);
    return offset + base_uv * tile_size;
}

fn recolor(sample: vec4<f32>, instance: ActorInstance) -> vec4<f32> {
    if (sample.a < 0.01) { return vec4<f32>(0.0); }
    let rgb = sample.rgb;
    if (distance(rgb, vec3<f32>(1.0, 0.0, 0.0)) < 0.01) {
        return vec4<f32>(instance.color_head.rgb, sample.a);
    }
    if (distance(rgb, vec3<f32>(0.0, 1.0, 0.0)) < 0.01) {
        return vec4<f32>(instance.color_body.rgb, sample.a);
    }
    if (distance(rgb, vec3<f32>(0.0, 0.0, 1.0)) < 0.01) {
        return vec4<f32>(instance.color_legs.rgb, sample.a);
    }
    if (distance(rgb, vec3<f32>(1.0, 1.0, 0.0)) < 0.01) {
        return vec4<f32>(instance.color_feet.rgb, sample.a);
    }
    return vec4<f32>(0.0);
}

fn alpha_blend(dst: vec4<f32>, src: vec4<f32>) -> vec4<f32> {
    let out_rgb = src.rgb * src.a + dst.rgb * (1.0 - src.a);
    let out_a = src.a + dst.a * (1.0 - src.a);
    return vec4<f32>(out_rgb, out_a);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    let instance = instances[in.instance_index];

    for (var i: u32 = 0u; i < instance.layer_count; i = i + 1u) {
        let uv = atlas_uv(in.uv, instance.sprite_ids[i]);
        let sample = textureSample(atlas_tex, atlas_smp, uv);

        if (i % 2u == 0u) {
            color = alpha_blend(color, sample);
        } else {
            let tint = recolor(sample, instance);
            let factor = mix(vec3<f32>(1.0), tint.rgb, sample.a);
            color = vec4<f32>(color.rgb * factor, color.a);
        }
    }
    return color;
}
