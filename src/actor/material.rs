use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct ActorInstance {
    pub moving: u32,
    pub direction: u32,
    pub addons: u32,
    pub mounted: u32,
    pub color_head: Vec4,
    pub color_body: Vec4,
    pub color_legs: Vec4,
    pub color_feet: Vec4,
    pub time_offset: f32,
    pub _pad: Vec3,
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct ActorParams {
    pub atlas_grid: Vec2,
    pub pattern_x: UVec2,
    pub pattern_y: UVec2,
    pub pattern_z: UVec2,
    pub layers: UVec2,
    pub phase_count: UVec2,
    pub phase_duration: f32,
    pub _pad: Vec3,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct ActorMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,

    #[uniform(2)]
    pub params: ActorParams,

    #[storage(3, read_only)]
    pub still_indexes: Handle<ShaderStorageBuffer>,

    #[storage(4, read_only)]
    pub moving_indexes: Handle<ShaderStorageBuffer>,

    #[storage(5, read_only)]
    pub instances: Handle<ShaderStorageBuffer>,
}

impl Material2d for ActorMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/actor.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/actor.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
