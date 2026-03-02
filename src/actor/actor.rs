use bevy::mesh::MeshTag;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

use crate::conf::z_order::ACTOR_Z_OFFSET;
use crate::core::{Appearances, SpriteConfig, SpriteSheet};

use crate::actor::assets::{Outfit, Outfits};
use crate::actor::colors::COLOR_TABLE;
use crate::actor::material::{ActorInstance, ActorMaterial, ActorParams};
use crate::actor::movement::Moving;
use crate::map::TilePosition;

#[derive(Resource, Default, Debug)]
pub struct LoadedMaterials {
    materials: HashMap<u32, (Handle<Mesh>, Handle<ActorMaterial>)>,
}

#[derive(Resource, Default, Debug)]
pub struct ActorInstances {
    in_use: Vec<bool>,
    data: Vec<ActorInstance>,
    buffer: Handle<ShaderStorageBuffer>,
}

impl ActorInstances {
    fn alloc_index(&mut self) -> u32 {
        for (i, in_use) in self.in_use.iter().enumerate() {
            if !in_use {
                self.in_use[i] = true;
                return i as u32;
            }
        }

        self.data.push(ActorInstance::default());
        self.in_use.push(true);
        return (self.data.len() - 1) as u32;
    }

    fn dealloc_index(&mut self, index: u32) {
        self.in_use[index as usize] = false;
        self.trim();
    }

    fn trim(&mut self) {
        while let Some(&in_use) = self.in_use.last() {
            if in_use {
                break;
            }
            self.in_use.pop();
            self.data.pop();
        }
    }
}

#[derive(Event, Debug)]
pub struct RemoveActor {
    pub entity: Entity,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FacingDirection {
    #[default]
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl From<FacingDirection> for u32 {
    fn from(value: FacingDirection) -> Self {
        value as u32
    }
}

impl From<FacingDirection> for usize {
    fn from(value: FacingDirection) -> Self {
        value as usize
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Mounted {
    #[default]
    Unmounted = 0,
    Mounted = 1,
}

impl From<Mounted> for u32 {
    fn from(value: Mounted) -> Self {
        value as u32
    }
}

#[derive(Component, Debug, Default)]
pub struct Actor {
    // pub outfit_id: u32,
    pub direction: FacingDirection,
    pub addons: u32,
    pub mounted: Mounted,
    pub color_head: u32,
    pub color_body: u32,
    pub color_legs: u32,
    pub color_feet: u32,
    pub speed: u32,
    pub box_size: [f32; 2],
    pub boxes: [[Rect; 4]; 2],
}

pub fn init_instances_buffer(
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let instances = ActorInstances {
        in_use: Vec::new(),
        data: Vec::new(),
        buffer: buffers.add(ShaderStorageBuffer::default()),
    };
    commands.insert_resource(instances);
}

pub fn spawn_actor(
    commands: &mut Commands,
    loaded_materials: &mut LoadedMaterials,
    materials: &mut Assets<ActorMaterial>,
    meshes: &mut Assets<Mesh>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    instances: &mut ActorInstances,
    appearances: &Appearances,
    outfits: &Outfits,
    time: &Time,
    outfit_id: u32,
    color_head: u32,
    color_body: u32,
    color_legs: u32,
    color_feet: u32,
    speed: u32,
    position: TilePosition,
) -> Entity {
    let outfit = outfits.outfits.get(&outfit_id).unwrap();
    let sheet = appearances.sheets.get(&outfit.sprite_group).unwrap();
    let still_sprite = appearances
        .sprite_configs
        .get(&outfit.still_sprite_id)
        .unwrap();
    let moving_sprite = appearances
        .sprite_configs
        .get(&outfit.moving_sprite_id)
        .unwrap();
    if !loaded_materials.materials.contains_key(&outfit.id) {
        init_material(
            outfit,
            sheet,
            still_sprite,
            moving_sprite,
            materials,
            meshes,
            buffers,
            loaded_materials,
            instances,
        );
    }

    let (mesh, material) = loaded_materials.materials.get(&outfit.id).unwrap();
    let index = instances.alloc_index();
    let instance = &mut instances.data[index as usize];
    instance.time_offset = time.elapsed_secs_wrapped();

    let actor = Actor {
        // outfit_id,
        color_head,
        color_body,
        color_feet,
        color_legs,
        speed,
        box_size: [still_sprite.box_size, moving_sprite.box_size],
        boxes: [
            still_sprite.boxes.clone().try_into().unwrap(),
            moving_sprite.boxes.clone().try_into().unwrap(),
        ],
        ..default()
    };

    let world_position = position.to_world();
    let entity = commands
        .spawn((
            actor,
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
            MeshTag(index),
            position,
            Transform::from_xyz(
                world_position.x,
                world_position.y,
                world_position.z + ACTOR_Z_OFFSET,
            ),
        ))
        .id();
    entity
}

fn init_material(
    outfit: &Outfit,
    sheet: &SpriteSheet,
    still_sprite: &SpriteConfig,
    moving_sprite: &SpriteConfig,
    materials: &mut Assets<ActorMaterial>,
    meshes: &mut Assets<Mesh>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    loaded_materials: &mut LoadedMaterials,
    instances: &ActorInstances,
) {
    let params = ActorParams {
        atlas_grid: sheet.grid_size,
        pattern_x: UVec2::new(still_sprite.pattern_x, moving_sprite.pattern_x),
        pattern_y: UVec2::new(still_sprite.pattern_y, moving_sprite.pattern_y),
        pattern_z: UVec2::new(still_sprite.pattern_z, moving_sprite.pattern_z),
        layers: UVec2::new(still_sprite.layers, moving_sprite.layers),
        phase_count: UVec2::new(
            still_sprite.animation.total_animation_phases(),
            moving_sprite.animation.total_animation_phases(),
        ),
        phase_duration: 0.1,
        _pad: Vec3::ZERO,
    };

    let material_handle = materials.add(ActorMaterial {
        texture: sheet.texture.clone(),
        params,
        still_indexes: buffers.add(ShaderStorageBuffer::from(&still_sprite.sprite_ids)),
        moving_indexes: buffers.add(ShaderStorageBuffer::from(&moving_sprite.sprite_ids)),
        instances: instances.buffer.clone(),
    });

    let mesh = Mesh::from(Rectangle::new(64.0, 64.0));
    let mesh_handle = meshes.add(mesh);
    loaded_materials
        .materials
        .insert(outfit.id, (mesh_handle, material_handle));
}

pub fn on_remove_actor(
    event: On<RemoveActor>,
    mut commands: Commands,
    mut instances: ResMut<ActorInstances>,
    actor_q: Query<&MeshTag, With<Actor>>,
) {
    let Ok(tag) = actor_q.get(event.entity) else {
        return;
    };
    instances.dealloc_index(tag.0);
    commands.entity(event.entity).despawn();
}

pub fn upload_instance_buffer(
    instances: Res<ActorInstances>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    loaded_materials: Res<LoadedMaterials>,
    mut materials: ResMut<Assets<ActorMaterial>>,
) {
    if !instances.is_changed() {
        return;
    }

    if let Some(ssb) = buffers.get_mut(&instances.buffer) {
        ssb.set_data(&instances.data);
    }

    for (_, mat) in loaded_materials.materials.values() {
        // set material as changed so buffer gets updated in the pipeline
        let _ = materials.get_mut(mat).unwrap();
    }
}

pub fn update_actor_instances(
    actors_q: Query<(&Actor, &MeshTag, Option<&Moving>), Changed<Actor>>,
    mut instances: ResMut<ActorInstances>,
) {
    for (actor, tag, moving) in actors_q {
        let index = tag.0;
        let instance = &mut instances.data[index as usize];
        instance.moving = if moving.is_some() { 1 } else { 0 };
        instance.direction = actor.direction.into();
        instance.mounted = actor.mounted.into();
        instance.addons = actor.addons;
        instance.color_head = COLOR_TABLE[actor.color_head as usize];
        instance.color_body = COLOR_TABLE[actor.color_body as usize];
        instance.color_legs = COLOR_TABLE[actor.color_legs as usize];
        instance.color_feet = COLOR_TABLE[actor.color_feet as usize];
        instance.bounding_square = actor.box_size[instance.moving as usize];
        let bbox = &actor.boxes[instance.moving as usize][actor.direction as usize];
        instance.bbox_min = bbox.min.clone();
        instance.bbox_size = bbox.max.clone();
    }
}

pub fn actor_rect(actors_q: Query<(&Transform, &Actor, Option<&Moving>)>, mut gizmos: Gizmos) {
    for (pos, actor, moving) in &actors_q {
        gizmos.circle_2d(pos.translation.truncate(), 2.0, Color::srgb(1.0, 0.0, 0.0));

        gizmos.rect_2d(
            pos.translation.truncate(),
            Vec2::splat(64.0),
            Color::srgb(0.0, 0.5, 1.0),
        );

        let mesh_start = pos.translation.truncate();
        let moving = if moving.is_some() { 1 } else { 0 } as usize;
        let iso =
            mesh_start + (actor.boxes[moving][actor.direction as usize].min * Vec2::new(0.5, -0.5));
        let bbox_size = actor.boxes[moving][actor.direction as usize].max;

        gizmos.rect_2d(iso, bbox_size, Color::srgb(1.0, 1.0, 0.0));
    }
}
