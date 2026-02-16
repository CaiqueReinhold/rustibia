#import bevy_sprite::mesh2d_view_bindings::globals
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct ActorParams {
    atlas_grid: vec2<f32>,

    pattern_x: u32,
    pattern_y: u32,
    pattern_z: u32,
    layers: u32,

    direction: u32,
    addon: u32,
    mounted: u32,
    _pad0: u32,

    phase_count: u32,
    _pad1: vec3<u32>,
    phase_duration: f32,
    time_offset: f32,
    _pad2: vec2<f32>,

    color0: vec4<f32>,
    color1: vec4<f32>,
    color2: vec4<f32>,
    color3: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var atlas_tex: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var atlas_smp: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> params: ActorParams;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<storage, read> sprite_indexes: array<u32>;

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

fn recolor(sample: vec4<f32>) -> vec4<f32> {
    if (sample.a < 0.01) {
        return vec4<f32>(0.0);
    }

    let rgb = sample.rgb;

    if (distance(rgb, vec3<f32>(1.0, 0.0, 0.0)) < 0.01) {
        return vec4<f32>(params.color0.rgb, sample.a);
    }
    if (distance(rgb, vec3<f32>(0.0, 1.0, 0.0)) < 0.01) {
        return vec4<f32>(params.color1.rgb, sample.a);
    }
    if (distance(rgb, vec3<f32>(0.0, 0.0, 1.0)) < 0.01) {
        return vec4<f32>(params.color2.rgb, sample.a);
    }
    if (distance(rgb, vec3<f32>(1.0, 1.0, 0.0)) < 0.01) {
        return vec4<f32>(params.color3.rgb, sample.a);
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
    params: ActorParams
) -> u32 {
    return (
        (
            (phase * params.pattern_z + params.mounted)
            * params.pattern_y + addon
        )
        * params.pattern_x + params.direction
    )
    * params.layers
    + layer;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    let time = globals.time;
    let t = time - params.time_offset;
    let p = floor(t / params.phase_duration);
    let phase = u32(p) % params.phase_count;

    for (var addon: u32 = 0u; addon < params.pattern_y; addon = addon + 1u) {
        for (var layer: u32 = 0u; layer < params.layers; layer =layer + 1u) {
            if (addon > 0 && (params.addon & addon) == 0) {
                continue;
            }
            let index = compute_index(phase, layer, addon, params);
            let atlas_index = u32(sprite_indexes[index]);
            let uv = atlas_uv(in.uv, atlas_index);
            let sample = textureSample(atlas_tex, atlas_smp, uv);

            var layer_color: vec4<f32>;

            if (layer % 2u == 0u) {
                color = alpha_blend(color, sample);
            } else {
                let tint = recolor(sample);
                let factor = mix(vec3<f32>(1.0), tint.rgb, sample.a);
                color = vec4<f32>(color.rgb * factor, color.a);
            }
        }
    }

    return color;
}