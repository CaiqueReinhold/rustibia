use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct ItemInstance {
    pub sprite_id: u32,
    pub _pad: u32, // required std430 alignment padding before vec2
    pub bbox_min: Vec2,
    pub bbox_size: Vec2,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct ItemMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,

    #[uniform(2)]
    pub atlas_grid: Vec2,

    #[storage(3, read_only)]
    pub instances: Handle<ShaderStorageBuffer>,

    #[uniform(4)]
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
