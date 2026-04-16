#import bevy_sprite::mesh2d_view_bindings::globals
#import bevy_sprite::mesh2d_functions as mesh_functions

struct ActorParams {
    atlas_grid: vec2<f32>,
    pattern_x: vec2<u32>,
    pattern_y: vec2<u32>,
    pattern_z: vec2<u32>,
    layers: vec2<u32>
}

struct ActorInstance {
    moving: u32,
    direction: u32,
    addons: u32,
    mounted: u32,
    color_head: vec4<f32>,
    color_body: vec4<f32>,
    color_legs: vec4<f32>,
    color_feet: vec4<f32>,
    time_offset: f32,
    bounding_square: f32,
    bbox_min: vec2<f32>,
    bbox_size: vec2<f32>,
    moving_progress: f32,
    phase_count: u32,
    phase_duration: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var atlas_tex: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var atlas_smp: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> params: ActorParams;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<storage, read> still_indexes: array<u32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var<storage, read> moving_indexes: array<u32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var<storage, read> instances: array<ActorInstance>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) instance_index: u32,
    @location(6) phase: u32,
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

    var world_from_local =
        mesh_functions::get_world_from_local(instance_index);
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

fn get_animation_phase(instance: ActorInstance) -> u32 {
    if (instance.moving == 1u) {
        let phase = u32(instance.moving_progress * f32(instance.phase_count));
        return phase;
    }

    let t = globals.time - instance.time_offset;
    let p = floor(t / instance.phase_duration);
    let phase = u32(p) % instance.phase_count;
    return phase;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let inst_index = mesh_functions::get_tag(vertex.instance_index);
    let inst = instances[inst_index];
    let bbox_min = inst.bbox_min;
    let bbox_size = inst.bbox_size;

    out.world_position = calculate_world_pos(
        vertex.position,
        64.0,
        bbox_min,
        bbox_size,
        vertex.instance_index
    );
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
    out.uv = adjust_uv_to_bbox(vertex.uv, 64.0, bbox_min, bbox_size);
    out.instance_index = inst_index;
    out.phase = get_animation_phase(inst);
    
    return out;
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

fn recolor(sample: vec4<f32>, instance: ActorInstance) -> vec4<f32> {
    if (sample.a < 0.01) {
        return vec4<f32>(0.0);
    }

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

fn compute_index(
    phase: u32,
    layer: u32,
    addon: u32,
    pattern_z: u32,
    pattern_y: u32,
    pattern_x: u32,
    layers: u32,
    instance: ActorInstance
) -> u32 {
    return (
        (
            (phase * pattern_z + instance.mounted)
            * pattern_y + addon
        )
        * pattern_x + instance.direction
    )
    * layers + layer;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);

    let instance = instances[in.instance_index];
    let pattern_x = params.pattern_x[instance.moving];
    let pattern_y = params.pattern_y[instance.moving];
    let pattern_z = params.pattern_z[instance.moving];
    let layers = params.layers[instance.moving];

    for (var addon: u32 = 0u; addon < pattern_y; addon = addon + 1u) {
        for (var layer: u32 = 0u; layer < layers; layer = layer + 1u) {
            if (addon > 0 && (instance.addons & addon) == 0) {
                continue;
            }

            let index = compute_index(
                in.phase,
                layer,
                addon,
                pattern_z,
                pattern_y,
                pattern_x,
                layers,
                instance
            );
            var atlas_index = 0u;
            if (instance.moving == 0u) {
                atlas_index = u32(still_indexes[index]);
            } else {
                atlas_index = u32(moving_indexes[index]);
            }
            let uv = atlas_uv(in.uv, atlas_index);
            let sample = textureSample(atlas_tex, atlas_smp, uv);

            if (layer % 2u == 0u) {
                color = alpha_blend(color, sample);
            } else {
                let tint = recolor(sample, instance);
                let factor = mix(vec3<f32>(1.0), tint.rgb, sample.a);
                color = vec4<f32>(color.rgb * factor, color.a);
            }
        }
    }

    return color;
}