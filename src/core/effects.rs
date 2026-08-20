use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};

use crate::core::InstanceManager;

/// One effect's slot in the shader storage buffer.
///
/// Byte-identical to `ItemInstance`: both describe a single-layer atlas sprite
/// with a bounding box and a shift, which is all `shaders/items.wgsl` consumes.
#[repr(C)]
#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectInstance {
    pub sprite_id: u32,
    pub _pad: u32, // required std430 alignment padding before vec2
    pub bbox_min: Vec2,
    pub bbox_size: Vec2,
    pub shift: Vec2,
}

/// Effects reuse `shaders/items.wgsl` rather than copying it: that shader is
/// generic over "single-layer atlas sprite", and is named after its first caller
/// rather than its only possible one.
///
/// A separate material type, instead of importing `ItemMaterial`, keeps the
/// effect system independent of `ItemsPlugin` — and avoids a second
/// `Material2dPlugin` registration for the same type, which panics.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct EffectMaterial {
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

impl Material2d for EffectMaterial {
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

/// A newtype so this manager and the item plugin's are two distinct ECS
/// resources rather than one shared by accident — they are keyed by type.
#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct EffectInstances(InstanceManager<EffectInstance>);

/// One buffer for every effect on screen, and one mesh and material per atlas
/// sheet group, built on first use. There are seven groups.
#[derive(Resource, Debug, Default)]
pub struct EffectMaterials {
    by_group: HashMap<String, (Handle<Mesh>, Handle<EffectMaterial>)>,
    buffer: Handle<ShaderStorageBuffer>,
}

pub fn setup_resources(mut commands: Commands, mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    commands.insert_resource(EffectMaterials {
        by_group: HashMap::new(),
        buffer: buffers.add(ShaderStorageBuffer::new(&[0], RenderAssetUsages::all())),
    });
}
