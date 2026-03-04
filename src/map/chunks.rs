use std::collections::{HashMap, HashSet};

use crate::actor::Player;
use crate::conf::map::{CHUNK_LOAD_RADIUS, CHUNK_SIZE, TILE_SIZE};
use crate::core::{Appearances, SpriteConfig};
use crate::map::material::TerrainMaterial;
use crate::map::material::{ATTRIBUTE_FRAME_COUNT, ATTRIBUTE_LOOKUP_INDEX, ATTRIBUTE_PATTERNS};
use crate::map::TileChanged;
use crate::map::{
    map::Map,
    position::{ChunkPosition, TilePosition},
};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::sprite_render::AlphaMode2d;

#[derive(Resource, Default, Debug)]
pub struct LoadedChunks {
    player_current_chunk: ChunkPosition,
    chunks: HashSet<ChunkPosition>,
}

#[derive(Resource, Default, Debug)]
pub struct LoadedMaterials {
    materials: HashMap<String, Handle<TerrainMaterial>>,
    lookups: HashMap<String, HashMap<u32, u32>>,
}

#[derive(Event)]
pub struct PlayerChunkChanged {
    current_chunk: ChunkPosition,
}

pub fn player_chunk_changed(
    mut commands: Commands,
    player_q: Query<&TilePosition, With<Player>>,
    loaded: Res<LoadedChunks>,
) {
    let Ok(player_pos) = player_q.single() else {
        return;
    };

    let player_chunk = ChunkPosition::from_tile(player_pos);
    if loaded.player_current_chunk != player_chunk {
        commands.trigger(PlayerChunkChanged {
            current_chunk: player_chunk,
        });
    }
}

pub fn update_visible_chunks(
    event: On<PlayerChunkChanged>,
    mut commands: Commands,
    mut loaded: ResMut<LoadedChunks>,
    world: Res<Map>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut loaded_materials: ResMut<LoadedMaterials>,
    appearances: Res<Appearances>,
    existing: Query<(Entity, &ChunkPosition)>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    time: Res<Time>,
) {
    let mut wanted = HashSet::new();

    for floor in 0..world.floors {
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
                &appearances,
                pos,
                &mut meshes,
                &mut loaded_materials,
                &mut materials,
                &mut buffers,
                &time,
            );
            loaded.chunks.insert(pos.clone());
        }
    }

    for (entity, chunk) in &existing {
        if !wanted.contains(&chunk) {
            commands.entity(entity).despawn();
            loaded.chunks.remove(&chunk);
        }
    }

    loaded.player_current_chunk = event.current_chunk.clone();
}

fn spawn_chunk(
    commands: &mut Commands,
    map: &Map,
    appearances: &Appearances,
    position: &ChunkPosition,
    meshes: &mut Assets<Mesh>,
    loaded_materials: &mut LoadedMaterials,
    materials: &mut Assets<TerrainMaterial>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    time: &Time,
) {
    let mut ground: Vec<(TilePosition, &SpriteConfig)> =
        Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);
    let mut borders: Vec<(TilePosition, &SpriteConfig)> =
        Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);

    let start = position.start_position();
    for dx in 0..CHUNK_SIZE {
        for dy in 0..CHUNK_SIZE {
            let tile_pos = TilePosition::new(start.x + dx, start.y + dy, start.floor);

            if let Some(tile) = map.tiles.get(&tile_pos) {
                if let Some(g) = &tile.ground {
                    let ground_sprite = appearances.sprite_configs.get(&g.sprite_id).unwrap();
                    ground.push((tile_pos.clone(), ground_sprite));
                }
                if let Some(b) = &tile.border {
                    let border_sprite = appearances.sprite_configs.get(&b.sprite_id).unwrap();
                    borders.push((tile_pos.clone(), border_sprite));
                }
                if !tile.items.is_empty() {
                    commands.trigger(TileChanged {
                        position: tile_pos.clone(),
                    });
                }
            }
        }
    }

    if ground.len() == 0 && borders.len() == 0 {
        return;
    }

    let entity = commands
        .spawn((
            position.clone(),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
        ))
        .id();

    if ground.len() > 0 {
        let group = &ground.first().unwrap().1.group;

        if !loaded_materials.materials.contains_key(group) {
            init_material(
                group,
                appearances,
                materials,
                buffers,
                loaded_materials,
                time,
                AlphaMode2d::Opaque,
            );
        }

        let material = loaded_materials.materials.get(group).unwrap();
        let material_lookup = loaded_materials.lookups.get(group).unwrap();

        let mesh = build_chunk_mesh(ground, material_lookup);
        commands.entity(entity).with_child((
            Transform::default(),
            GlobalTransform::default(),
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(material.clone()),
        ));
    }

    if borders.len() > 0 {
        let group = &borders.first().unwrap().1.group;

        if !loaded_materials.materials.contains_key(group) {
            init_material(
                group,
                appearances,
                materials,
                buffers,
                loaded_materials,
                time,
                AlphaMode2d::Blend,
            );
        }
        let material = loaded_materials.materials.get(group).unwrap();
        let material_lookup = loaded_materials.lookups.get(group).unwrap();

        let mesh = build_chunk_mesh(borders, material_lookup);
        commands.entity(entity).with_child((
            Transform::default(),
            GlobalTransform::default(),
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(material.clone()),
        ));
    }
}

fn build_chunk_mesh(
    tiles: Vec<(TilePosition, &SpriteConfig)>,
    lookup_map: &HashMap<u32, u32>,
) -> Mesh {
    let cap = (CHUNK_SIZE * CHUNK_SIZE * 4) as usize;
    let mut positions = Vec::<[f32; 3]>::with_capacity(cap);
    let mut uvs = Vec::<[f32; 2]>::with_capacity(cap);
    let mut lookup_index = Vec::<u32>::with_capacity(cap);
    let mut patterns = Vec::<[u32; 4]>::with_capacity(cap);
    let mut frame_counts = Vec::<u32>::with_capacity(cap);
    let mut indices = Vec::<u32>::with_capacity(cap);
    let mut quad_index: u32 = 0;

    for (pos, tile) in tiles.iter() {
        push_quad(
            &mut positions,
            &mut uvs,
            &mut lookup_index,
            &mut frame_counts,
            &mut patterns,
            &mut indices,
            &mut quad_index,
            pos,
            *tile,
            lookup_map,
        );
    }

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(ATTRIBUTE_LOOKUP_INDEX, lookup_index)
    .with_inserted_attribute(ATTRIBUTE_FRAME_COUNT, frame_counts)
    .with_inserted_attribute(ATTRIBUTE_PATTERNS, patterns)
    .with_inserted_indices(Indices::U32(indices));

    mesh
}

fn push_quad(
    positions: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    lookup_index: &mut Vec<u32>,
    frame_counts: &mut Vec<u32>,
    patterns: &mut Vec<[u32; 4]>,
    indices: &mut Vec<u32>,
    quad_index: &mut u32,
    t_pos: &TilePosition,
    ground: &SpriteConfig,
    lookup_map: &HashMap<u32, u32>,
) {
    let i = *quad_index * 4;

    let w_pos = t_pos.to_world();
    positions.extend_from_slice(&[
        [w_pos.x, w_pos.y, w_pos.z],
        [w_pos.x + TILE_SIZE, w_pos.y, w_pos.z],
        [w_pos.x + TILE_SIZE, w_pos.y - TILE_SIZE, w_pos.z],
        [w_pos.x, w_pos.y - TILE_SIZE, w_pos.z],
    ]);

    uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

    let index = match lookup_map.get(&ground.id) {
        Some(i) => *i,
        None => {
            panic!("failed");
        }
    };
    let animation_frames = ground.animation.total_animation_phases();
    for _ in 0..4 {
        let y_pat = t_pos.y % ground.pattern_y;
        let x_pat = t_pos.x % ground.pattern_x;
        lookup_index.push(index);
        frame_counts.push(animation_frames);
        patterns.push([ground.pattern_x, ground.pattern_y, x_pat, y_pat]);
    }

    indices.extend_from_slice(&[i, i + 2, i + 1, i, i + 3, i + 2]);

    *quad_index += 1;
}

fn init_material(
    group: &String,
    appearances: &Appearances,
    materials: &mut Assets<TerrainMaterial>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    loaded_materials: &mut LoadedMaterials,
    time: &Time,
    alpha_mode: AlphaMode2d,
) {
    let mut lookup_map: HashMap<u32, u32> = HashMap::new();
    let mut animation_frame_lookup: Vec<u32> = Vec::new();

    for config in appearances.get_group(group) {
        lookup_map.insert(config.id, animation_frame_lookup.len() as u32);
        animation_frame_lookup.extend_from_slice(config.sprite_ids.as_slice());
    }

    let sheet = appearances.sheets.get(group).unwrap();

    let material = materials.add(TerrainMaterial {
        atlas: sheet.texture.clone(),
        time_offset: time.elapsed_secs_wrapped(),
        atals_grid: sheet.grid_size,
        animated_sprite_lookup: buffers
            .add(ShaderStorageBuffer::from(animation_frame_lookup.as_slice())),
        alpha_mode,
    });
    loaded_materials.materials.insert(group.clone(), material);
    loaded_materials.lookups.insert(group.clone(), lookup_map);
}

pub fn draw_tile_grid(mut gizmos: Gizmos) {
    // Vertical lines
    for x in -40000..=40000 {
        let world_x = x as f32 * TILE_SIZE;

        gizmos.line_2d(
            Vec2::new(world_x, -80000.0),
            Vec2::new(world_x, 80000.0),
            Color::srgb(1.0, 1., 1.),
        );
    }

    // Horizontal lines
    for y in -40000..=40000 {
        let world_y = y as f32 * TILE_SIZE;

        gizmos.line_2d(
            Vec2::new(-80000.0, world_y),
            Vec2::new(80000.0, world_y),
            Color::srgb(1.0, 1., 1.),
        );
    }
}
