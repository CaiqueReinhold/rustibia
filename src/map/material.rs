use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::mesh::{Mesh, MeshVertexAttribute, VertexFormat};

use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey};

pub const ATTRIBUTE_LOOKUP_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("lookup_index", 1001, VertexFormat::Uint32);
pub const ATTRIBUTE_FRAME_COUNT: MeshVertexAttribute =
    MeshVertexAttribute::new("frame_count", 1002, VertexFormat::Uint32);
pub const ATTRIBUTE_PATTERNS: MeshVertexAttribute =
    MeshVertexAttribute::new("patterns", 1003, VertexFormat::Uint32x4);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub atlas: Handle<Image>,

    #[uniform(2)]
    pub time_offset: f32,

    #[uniform(3)]
    pub atals_grid: Vec2,

    #[storage(4, read_only)]
    pub animated_sprite_lookup: Handle<ShaderStorageBuffer>,

    pub alpha_mode: AlphaMode2d,
}

impl Material2d for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.alpha_mode
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_LOOKUP_INDEX.at_shader_location(3),
            ATTRIBUTE_FRAME_COUNT.at_shader_location(4),
            ATTRIBUTE_PATTERNS.at_shader_location(5),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
