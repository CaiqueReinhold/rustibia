use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct ItemInstance {
    pub phase_count: u32,
    pub phase_duration: f32,
    pub time_offset: f32,
    pub lookup_offset: u32,
    pub pattern_x: u32,
    pub pattern_y: u32,
    pub pattern_z: u32,
    pub value_x: u32,
    pub value_y: u32,
    pub value_z: u32,
    pub bbox_min: Vec2,
    pub bbox_size: Vec2,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct ItemMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,

    #[uniform(2)]
    pub time_offset: f32,

    #[uniform(3)]
    pub atlas_grid: Vec2,

    #[storage(4, read_only)]
    pub sprite_lookup: Handle<ShaderStorageBuffer>,

    #[storage(5, read_only)]
    pub instances: Handle<ShaderStorageBuffer>,

    #[uniform(6)]
    pub mesh_size: Vec2,
}

impl Material2d for ItemMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/items.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/items.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
