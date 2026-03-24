use crate::{
    conf::z_order::TOP_Z_OFFSET,
    core::{Appearances, InstanceManager, SpriteAnimation, SpriteConfig},
    items::{
        item::ItemFlag,
        material::{ItemInstance, ItemMaterial},
        Item, ItemId,
    },
    map::{Map, TilePosition},
};
use bevy::{asset::RenderAssetUsages, render::storage::ShaderStorageBuffer};
use bevy::{mesh::MeshTag, prelude::*};
use std::collections::{HashMap, VecDeque};

#[derive(Component)]
pub struct SpawnedItem;

#[derive(Resource, Debug, Default)]
pub struct ItemStacks {
    pub occupied_tiles: HashMap<TilePosition, Entity>,
}

#[derive(Resource, Debug, Default)]
pub struct LoadedMaterials {
    materials: HashMap<String, (Handle<Mesh>, Handle<ItemMaterial>)>,
    lookups: HashMap<String, HashMap<ItemId, u32>>,
    buffer: Handle<ShaderStorageBuffer>,
}

#[derive(Resource, Debug, Default)]
pub struct ChangedTileQueue {
    pub changed_positions: VecDeque<TilePosition>,
}

pub fn init_material_buffer(
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let loaded_materials = LoadedMaterials {
        materials: HashMap::new(),
        lookups: HashMap::new(),
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
    mut stacks: ResMut<ItemStacks>,
    mut instances: ResMut<InstanceManager<ItemInstance>>,
    mut loaded_materials: ResMut<LoadedMaterials>,
    mut materials: ResMut<Assets<ItemMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    map: Res<Map>,
    appearances: Res<Appearances>,
    time: Res<Time>,
) {
    while let Some(position) = queue.changed_positions.pop_front() {
        if let Some(entity) = stacks.occupied_tiles.remove(&position) {
            commands.entity(entity).despawn();
        }

        if let Some(items) = map.get_items(&position) {
            let world_pos = position.to_world();
            let parent = commands
                .spawn((Transform::from_xyz(world_pos.x, world_pos.y, world_pos.z),))
                .id();
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
                    &mut buffers,
                    &time,
                );
                commands.entity(parent).add_child(item_entity);
            }
            stacks.occupied_tiles.insert(position.clone(), parent);
        }
    }
}

fn spawn_item(
    item: &Item,
    position: &TilePosition,
    stack_index: usize,
    commands: &mut Commands,
    instances: &mut InstanceManager<ItemInstance>,
    loaded_materials: &mut LoadedMaterials,
    appearances: &Appearances,
    materials: &mut Assets<ItemMaterial>,
    meshes: &mut Assets<Mesh>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    time: &Time,
) -> Entity {
    let sprite = appearances.get_item(item.config.id);

    if !loaded_materials.materials.contains_key(&sprite.group) {
        init_material(
            &sprite.group,
            materials,
            meshes,
            buffers,
            loaded_materials,
            appearances,
            time,
        );
    }

    let (mesh, material) = loaded_materials.materials.get(&sprite.group).unwrap();
    let index = instances.alloc_index();
    let instance = &mut instances.get_mut(index);
    init_instance(
        instance,
        sprite,
        time.elapsed_secs(),
        *loaded_materials
            .lookups
            .get(&sprite.group)
            .unwrap()
            .get(&sprite.id)
            .unwrap(),
        item.get_patterns(position, sprite),
    );

    let mut translation = Vec3::new(0.0, 0.0, 0.001 * stack_index as f32);
    if item.config.has_flag(ItemFlag::Top) {
        translation.z = TOP_Z_OFFSET;
    }
    let entity = commands
        .spawn((
            SpawnedItem,
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
            MeshTag(index),
            Transform::from_translation(translation),
        ))
        .id();
    entity
}

fn init_instance(
    instance: &mut ItemInstance,
    sprite: &SpriteConfig,
    time_offset_secs: f32,
    lookup_offset: u32,
    patterns: (u32, u32, u32),
) {
    instance.time_offset = time_offset_secs;
    instance.phase_duration = match &sprite.animation {
        SpriteAnimation::Static => 0.0,
        SpriteAnimation::Uniform { phase_duration, .. } => phase_duration.as_secs_f32(),
        _ => 0.0,
    };
    instance.phase_count = sprite.animation.total_animation_phases();
    instance.lookup_offset = lookup_offset;
    let (px, py, pz) = patterns;
    instance.pattern_x = sprite.pattern_x;
    instance.pattern_y = sprite.pattern_y;
    instance.pattern_z = sprite.pattern_z;
    instance.value_x = px;
    instance.value_y = py;
    instance.value_z = pz;
    if !sprite.boxes.is_empty() {
        let bbox = &sprite.boxes[px as usize];
        instance.bbox_min = bbox.min;
        instance.bbox_size = bbox.max;
    } else {
        instance.bbox_min = Vec2::new(0.0, 0.0);
        instance.bbox_size = Vec2::new(32.0, 32.0);
    }
    instance.bounding_square = sprite.box_size;
}

fn init_material(
    group: &String,
    materials: &mut Assets<ItemMaterial>,
    meshes: &mut Assets<Mesh>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    loaded_materials: &mut LoadedMaterials,
    appearances: &Appearances,
    time: &Time,
) {
    let mut lookup_map: HashMap<ItemId, u32> = HashMap::new();
    let mut animation_frame_lookup: Vec<u32> = Vec::new();

    for config in appearances.iter_group_items(group) {
        lookup_map.insert(config.id, animation_frame_lookup.len() as u32);
        animation_frame_lookup.extend_from_slice(config.sprite_ids.as_slice());
    }

    let sheet = appearances.get_sheet(group);
    let material_handle = materials.add(ItemMaterial {
        texture: sheet.texture.clone(),
        time_offset: time.elapsed_secs(),
        atlas_grid: sheet.grid_size,
        sprite_lookup: buffers.add(ShaderStorageBuffer::from(&animation_frame_lookup)),
        instances: loaded_materials.buffer.clone(),
    });

    let mesh = Mesh::from(Rectangle::new(64.0, 64.0));
    let mesh_handle = meshes.add(mesh);
    loaded_materials
        .materials
        .insert(group.clone(), (mesh_handle, material_handle));
    loaded_materials.lookups.insert(group.clone(), lookup_map);
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
        // set material as changed so buffer gets updated in the pipeline
        let _ = materials.get_mut(mat).unwrap();
    }
}
