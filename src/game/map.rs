use std::collections::HashSet;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::mesh::{Indices, Mesh, MeshVertexAttribute, PrimitiveTopology, VertexFormat};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin};

use crate::conf::map::{CHUNK_LOAD_RADIUS, CHUNK_SIZE, FLOOR_Z_OFFSET, TILE_SIZE};
use crate::data::map_assets::WorldMap;

use crate::data::{AppearanceData, State};
use crate::game::TilePosition;
use crate::ui::gameview::GameCamera;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<TerrainMaterial>::default())
            .init_resource::<LoadedChunks>()
            .add_systems(OnEnter(State::Ready), init_material)
            .add_observer(update_visible_chunks_on_change)
            .add_systems(
                Update,
                (fire_event, check_input).run_if(in_state(State::Ready)),
            );
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Default)]
pub struct ChunkPosition {
    cx: u32,
    cy: u32,
    cz: u32,
}

impl ChunkPosition {
    pub fn new(cx: u32, cy: u32, cz: u32) -> Self {
        ChunkPosition { cx, cy, cz }
    }

    pub fn from_tile_position(tile_pos: &TilePosition) -> Self {
        let pos = tile_pos.absolute();
        Self {
            cx: pos.x / CHUNK_SIZE,
            cy: pos.y / CHUNK_SIZE,
            cz: pos.z,
        }
    }

    pub fn to_world_position(&self) -> Vec3 {
        Vec3::new(
            (self.cx * CHUNK_SIZE) as f32 * TILE_SIZE,
            -((self.cy * CHUNK_SIZE) as f32) * TILE_SIZE,
            self.cz as f32 * FLOOR_Z_OFFSET,
        )
    }
}

#[derive(Event)]
pub struct PlayerChunkChanged {
    current_chunk: ChunkPosition,
}

#[derive(Resource, Default)]
pub struct LoadedChunks {
    pub player_current_chunk: ChunkPosition,
    pub chunks: HashSet<ChunkPosition>,
}

#[derive(Resource)]
pub struct TerrainMaterialHandle(Handle<ColorMaterial>);

#[derive(Component)]
pub struct TerrainChunk {
    pos: ChunkPosition,
}

/// Custom vertex attributes for terrain (must match shader @location).
const ATTRIBUTE_SPRITE_ID: MeshVertexAttribute =
    MeshVertexAttribute::new("sprite_id", 1001, VertexFormat::Uint32);
const ATTRIBUTE_FRAME_COUNT: MeshVertexAttribute =
    MeshVertexAttribute::new("frame_count", 1002, VertexFormat::Uint32);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub atlas: Handle<Image>,
}

impl Material2d for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_SPRITE_ID.at_shader_location(3),
            ATTRIBUTE_FRAME_COUNT.at_shader_location(4),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

fn init_material(
    mut commands: Commands,
    sheets: Res<AppearanceData>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut player_q: Query<&mut Transform, With<GameCamera>>,
) {
    let sheet = sheets.ground_sheets.get(&1).unwrap();
    // let material = TerrainMaterial {
    //     atlas: sheet.texture.clone(),
    // };
    let handle = materials.add(sheet.texture.clone());
    commands.insert_resource(TerrainMaterialHandle(handle));

    let mut transform = player_q.single_mut().unwrap();
    transform.translation = TilePosition::new(1028, 1028, 7).to_world_position();
}

fn check_input(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<&mut Transform, With<GameCamera>>,
) {
    let mut transform = player_q.single_mut().unwrap();
    let direction: Vec3;
    if keyboard.pressed(KeyCode::KeyW) {
        direction = Vec3::new(0.0, 1.0, 0.0);
    } else if keyboard.pressed(KeyCode::KeyS) {
        direction = Vec3::new(0.0, -1.0, 0.0);
    } else if keyboard.pressed(KeyCode::KeyA) {
        direction = Vec3::new(-1.0, 0.0, 0.0);
    } else if keyboard.pressed(KeyCode::KeyD) {
        direction = Vec3::new(1.0, 0.0, 0.0);
    } else {
        return;
    }

    transform.translation =
        transform.translation + (direction * Vec3::splat(0.5) * time.elapsed().as_secs_f32());
}

fn fire_event(
    mut commands: Commands,
    player_q: Query<&Transform, With<GameCamera>>,
    loaded: Res<LoadedChunks>,
) {
    let Ok(transform) = player_q.single() else {
        return;
    };

    let tile = TilePosition {
        x: transform.translation.x / 32.0,
        y: (transform.translation.y * -1.0) / 32.0,
        z: 7,
    };

    let chunk_pos = ChunkPosition::from_tile_position(&tile);

    if loaded.player_current_chunk != chunk_pos {
        commands.trigger(PlayerChunkChanged {
            current_chunk: chunk_pos,
        });
    }
}

fn update_visible_chunks_on_change(
    event: On<PlayerChunkChanged>,
    mut commands: Commands,
    mut loaded: ResMut<LoadedChunks>,
    existing: Query<(Entity, &TerrainChunk)>,
    world: Res<WorldMap>,
    sheets: Res<AppearanceData>,
    mut meshes: ResMut<Assets<Mesh>>,
    material_handle: Res<TerrainMaterialHandle>,
) {
    let mut wanted = HashSet::new();

    for floor in 0..world.depth {
        for dy in -(CHUNK_LOAD_RADIUS as i32)..=(CHUNK_LOAD_RADIUS as i32) {
            for dx in -(CHUNK_LOAD_RADIUS as i32)..=(CHUNK_LOAD_RADIUS as i32) {
                let cx = event.current_chunk.cx as i32 + dx;
                let cy = event.current_chunk.cy as i32 + dy;

                if cx >= 0 && cy >= 0 {
                    wanted.insert(ChunkPosition::new(cx as u32, cy as u32, floor));
                }
            }
        }
    }

    for pos in &wanted {
        if !loaded.chunks.contains(pos) {
            spawn_chunk(
                &mut commands,
                &world,
                pos.clone(),
                &mut meshes,
                &material_handle,
                &sheets,
            );
            loaded.chunks.insert(pos.clone());
        }
    }

    for (entity, chunk) in &existing {
        if !wanted.contains(&chunk.pos) {
            commands.entity(entity).despawn();
            loaded.chunks.remove(&chunk.pos);
        }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    map: &WorldMap,
    pos: ChunkPosition,
    meshes: &mut Assets<Mesh>,
    material: &TerrainMaterialHandle,
    sheets: &AppearanceData,
) {
    let Some(mesh) = build_chunk_mesh(map, &pos, sheets) else {
        return;
    };
    let world_pos = pos.to_world_position();

    commands.spawn((
        Transform::from_xyz(world_pos.x, world_pos.y, world_pos.z),
        GlobalTransform::default(),
        Mesh2d(meshes.add(mesh)),
        MeshMaterial2d(material.0.clone()),
    ));
}

fn build_chunk_mesh(
    world: &WorldMap,
    pos: &ChunkPosition,
    appearance: &AppearanceData,
) -> Option<Mesh> {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut sprite_ids = Vec::<u32>::new();
    let mut frame_counts = Vec::<u32>::new();
    let mut indices = Vec::<u32>::new();
    let mut quad_index: u32 = 0;

    let start_x = pos.cx * CHUNK_SIZE;
    let start_y = pos.cy * CHUNK_SIZE;

    for ty in 0..=CHUNK_SIZE {
        for tx in 0..=CHUNK_SIZE {
            let x = start_x + tx;
            let y = start_y + ty;

            let ground = world.ground.get(&(x, y, pos.cz));

            let px = tx as f32 * TILE_SIZE;
            let py = -(ty as f32) * TILE_SIZE;

            if let Some(ground) = ground {
                let atlas = appearance.ground_sheets.get(&1).unwrap();
                push_quad(
                    &mut positions,
                    &mut uvs,
                    &mut sprite_ids,
                    &mut frame_counts,
                    &mut indices,
                    &mut quad_index,
                    px,
                    py,
                    atlas.get_uv(ground.sprite_id),
                    0.0,
                    ground.sprite_id,
                    ground.frame_count,
                );
            }

            let border = world.borders.get(&(x, y, pos.cz));
            if let Some(border) = border {
                let atlas = appearance.ground_sheets.get(&1).unwrap();
                push_quad(
                    &mut positions,
                    &mut uvs,
                    &mut sprite_ids,
                    &mut frame_counts,
                    &mut indices,
                    &mut quad_index,
                    px,
                    py,
                    atlas.get_uv(border.sprite_id),
                    0.01,
                    border.sprite_id,
                    1,
                );
            }
        }
    }

    if positions.is_empty() {
        return None;
    }

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(ATTRIBUTE_SPRITE_ID, sprite_ids)
    .with_inserted_attribute(ATTRIBUTE_FRAME_COUNT, frame_counts)
    .with_inserted_indices(Indices::U32(indices));

    Some(mesh)
}

fn push_quad(
    positions: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    sprite_ids: &mut Vec<u32>,
    frame_counts: &mut Vec<u32>,
    indices: &mut Vec<u32>,
    quad_index: &mut u32,
    x: f32,
    y: f32,
    sprite_uv: Rect,
    z: f32,
    sprite_id: u32,
    frame_count: u32,
) {
    let i = *quad_index * 4;

    positions.extend_from_slice(&[
        [x, y, z],
        [x + TILE_SIZE, y, z],
        [x + TILE_SIZE, y - TILE_SIZE, z],
        [x, y - TILE_SIZE, z],
    ]);

    uvs.extend_from_slice(&[
        [sprite_uv.min.x, sprite_uv.min.y],
        [sprite_uv.max.x, sprite_uv.min.y],
        [sprite_uv.max.x, sprite_uv.max.y],
        [sprite_uv.min.x, sprite_uv.max.y],
    ]);

    for _ in 0..4 {
        sprite_ids.push(sprite_id);
        frame_counts.push(frame_count);
    }

    indices.extend_from_slice(&[i, i + 2, i + 1, i, i + 3, i + 2]);

    *quad_index += 1;
}
