use crate::{
    conf::z_order::{GROUND_PASS_OFFSET, TOP_Z_OFFSET},
    core::{Appearances, InstanceManager, SpriteAnimator, SpriteConfig},
    items::{
        item::ItemFlag,
        material::{ItemInstance, ItemMaterial},
        Item,
    },
    map::{FloorEntities, Map, Position},
};
use bevy::prelude::*;
use bevy::{asset::RenderAssetUsages, mesh::MeshTag, render::storage::ShaderStorageBuffer};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Component)]
pub struct SpawnedItem;

#[derive(Resource, Debug, Default)]
pub struct ItemState {
    pub occupied_tiles: HashMap<Position, Entity>,
}

#[derive(Resource, Debug, Default)]
pub struct LoadedMaterials {
    materials: HashMap<String, (Handle<Mesh>, Handle<ItemMaterial>)>,
    buffer: Handle<ShaderStorageBuffer>,
}

#[derive(Resource, Debug, Default)]
pub struct ChangedTileQueue {
    pub changed_positions: VecDeque<Position>,
}

pub fn setup_resources(mut commands: Commands, mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    let loaded_materials = LoadedMaterials {
        materials: HashMap::new(),
        buffer: buffers.add(ShaderStorageBuffer::new(&[0], RenderAssetUsages::all())),
    };
    commands.insert_resource(loaded_materials);
}

pub fn on_remove_item(
    event: On<Remove, SpawnedItem>,
    tag_q: Query<&MeshTag, With<SpawnedItem>>,
    mut instances: ResMut<InstanceManager<ItemInstance>>,
) {
    let Ok(tag) = tag_q.get(event.entity) else {
        return;
    };

    instances.dealloc_index(tag.0);
}

pub fn process_tile_changed(
    mut queue: ResMut<ChangedTileQueue>,
    mut commands: Commands,
    mut state: ResMut<ItemState>,
    mut instances: ResMut<InstanceManager<ItemInstance>>,
    mut loaded_materials: ResMut<LoadedMaterials>,
    mut materials: ResMut<Assets<ItemMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    map: Res<Map>,
    appearances: Res<Appearances>,
    floor_entities: Res<FloorEntities>,
) {
    while let Some(position) = queue.changed_positions.pop_front() {
        if let Some(entity) = state.occupied_tiles.remove(&position) {
            commands
                .entity(floor_entities.floors[position.z as usize])
                .detach_child(entity);
            commands.entity(entity).despawn();
        }

        if let Some(items) = map.get_items(&position) {
            let world_pos = position.to_world();
            let parent = commands
                .spawn((
                    Transform::from_xyz(world_pos.x, world_pos.y, world_pos.z),
                    Visibility::Inherited,
                ))
                .id();
            commands
                .entity(floor_entities.floors[position.z as usize])
                .add_child(parent);

            let mut elevation = 0.0;
            for (i, item) in items.enumerate() {
                let item_entity = spawn_item(
                    item,
                    &position,
                    i,
                    &mut commands,
                    &mut instances,
                    &mut loaded_materials,
                    &appearances,
                    &mut materials,
                    &mut meshes,
                    elevation,
                );
                commands.entity(parent).add_child(item_entity);

                if let Some(item_elev) = item.config.elevation {
                    elevation += item_elev as f32;
                    if elevation >= 24.0 {
                        elevation = 24.0;
                    }
                }
            }
            state.occupied_tiles.insert(position.clone(), parent);
        }
    }
}

fn spawn_item(
    item: &Item,
    position: &Position,
    stack_index: usize,
    commands: &mut Commands,
    instances: &mut InstanceManager<ItemInstance>,
    loaded_materials: &mut LoadedMaterials,
    appearances: &Appearances,
    materials: &mut Assets<ItemMaterial>,
    meshes: &mut Assets<Mesh>,
    elevation: f32,
) -> Entity {
    let sprite = appearances.get_item(item.config.id);
    let sheet = appearances.get_sheet(&sprite.group);

    if !loaded_materials.materials.contains_key(&sprite.group) {
        init_material(
            &sprite.group,
            materials,
            meshes,
            loaded_materials,
            appearances,
        );
    }

    let (mesh, material) = loaded_materials.materials.get(&sprite.group).unwrap();
    let index = instances.alloc_index();
    let instance = instances.get_mut(index);
    let patterns = item.get_patterns(position, &sprite);
    let (px, py, pz) = patterns;
    init_instance(instance, &sprite, patterns);

    let animator = SpriteAnimator::new(Arc::clone(&sprite), px, py, pz);
    instance.sprite_id = animator.current_sprite_ids[0];

    let z = if item.config.has_flag(ItemFlag::Top) {
        TOP_Z_OFFSET
    } else if item.config.has_flag(ItemFlag::Ground) || item.config.has_flag(ItemFlag::Border) {
        GROUND_PASS_OFFSET + 0.001 * stack_index as f32
    } else {
        0.001 * stack_index as f32
    };
    let shift_x = if sheet.sprite_size.x <= 32.0 {
        16.0
    } else {
        0.0
    };
    let shift_y = if sheet.sprite_size.y <= 32.0 {
        -16.0
    } else {
        0.0
    };
    let translation = Vec3::new(-elevation + shift_x, elevation + shift_y, z);

    commands
        .spawn((
            SpawnedItem,
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
            MeshTag(index),
            Transform::from_translation(translation),
            Visibility::Inherited,
            animator,
        ))
        .id()
}

fn init_instance(instance: &mut ItemInstance, sprite: &SpriteConfig, patterns: (u32, u32, u32)) {
    let (px, _py, _pz) = patterns;
    if !sprite.boxes.is_empty() {
        let bbox = &sprite.boxes[px as usize];
        instance.bbox_min = bbox.min;
        instance.bbox_size = bbox.max;
    } else {
        instance.bbox_min = Vec2::ZERO;
        instance.bbox_size = Vec2::new(32.0, 32.0);
    }
}

fn init_material(
    group: &str,
    materials: &mut Assets<ItemMaterial>,
    meshes: &mut Assets<Mesh>,
    loaded_materials: &mut LoadedMaterials,
    appearances: &Appearances,
) {
    let sheet = appearances.get_sheet(group);
    let material_handle = materials.add(ItemMaterial {
        texture: sheet.texture.clone(),
        atlas_grid: sheet.grid_size,
        mesh_size: sheet.sprite_size,
        instances: loaded_materials.buffer.clone(),
    });
    let mesh_handle = meshes.add(Mesh::from(Rectangle::new(
        sheet.sprite_size.x,
        sheet.sprite_size.y,
    )));
    loaded_materials
        .materials
        .insert(group.to_string(), (mesh_handle, material_handle));
}

pub fn update_item_instances(
    items_q: Query<(&SpriteAnimator, &MeshTag), (With<SpawnedItem>, Changed<SpriteAnimator>)>,
    mut instances: ResMut<InstanceManager<ItemInstance>>,
) {
    for (animator, tag) in &items_q {
        let instance = instances.get_mut(tag.0);
        instance.sprite_id = animator.current_sprite_ids[0];
    }
}

pub fn upload_instance_buffer(
    mut instances: ResMut<InstanceManager<ItemInstance>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    loaded_materials: Res<LoadedMaterials>,
    mut materials: ResMut<Assets<ItemMaterial>>,
) {
    if !instances.is_dirty() {
        return;
    }

    if let Some(ssb) = buffers.get_mut(&loaded_materials.buffer) {
        ssb.set_data(instances.get_buffer_data());
        instances.reset_dirty();
    }

    for (_, mat) in loaded_materials.materials.values() {
        let _ = materials.get_mut(mat).unwrap();
    }
}
