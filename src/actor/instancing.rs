use bevy::asset::RenderAssetUsages;
use bevy::mesh::MeshTag;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

use crate::actor::components::Actor;
use crate::conf::z_order::ACTOR_Z_OFFSET;
use crate::core::OutfitId;
use crate::core::{Appearances, InstanceManager, OutfitSprite, SpriteAnimation, SpriteSheet};

use crate::actor::{
    colors::COLOR_TABLE,
    material::{ActorInstance, ActorMaterial, ActorParams},
    movement::Moving,
};
use crate::map::Position;

#[derive(Resource, Default, Debug)]
pub struct LoadedMaterials {
    materials: HashMap<OutfitId, (Handle<Mesh>, Handle<ActorMaterial>)>,
    buffer: Handle<ShaderStorageBuffer>,
}

#[derive(Event, Debug)]
pub struct RemoveActor {
    pub entity: Entity,
}

pub fn init_instances_buffer(
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let loaded_materials = LoadedMaterials {
        materials: HashMap::new(),
        buffer: buffers.add(ShaderStorageBuffer::new(&[0], RenderAssetUsages::all())),
    };
    commands.insert_resource(loaded_materials);
}

pub fn spawn_actor(
    commands: &mut Commands,
    loaded_materials: &mut LoadedMaterials,
    materials: &mut Assets<ActorMaterial>,
    meshes: &mut Assets<Mesh>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    instances: &mut InstanceManager<ActorInstance>,
    appearances: &Appearances,
    time: &Time,
    outfit_id: OutfitId,
    color_head: u32,
    color_body: u32,
    color_legs: u32,
    color_feet: u32,
    speed: u16,
    addons: u32,
    position: Position,
) -> Entity {
    let outfit = appearances.get_outfit(outfit_id);
    let sheet = appearances.get_sheet(&outfit.still_sprite.group);

    if !loaded_materials.materials.contains_key(&outfit.id) {
        init_material(outfit, sheet, materials, meshes, buffers, loaded_materials);
    }

    let (mesh, material) = loaded_materials.materials.get(&outfit.id).unwrap();
    let index = instances.alloc_index();
    let instance = &mut instances.get_mut(index);
    instance.time_offset = time.elapsed_secs_wrapped();
    instance.phase_duration = match &outfit.still_sprite.animation {
        SpriteAnimation::Static => 0.1,
        SpriteAnimation::Uniform { phase_duration, .. } => phase_duration.as_secs_f32(),
        _ => 0.1,
    };

    let actor = Actor {
        // outfit_id,
        addons,
        color_head,
        color_body,
        color_feet,
        color_legs,
        speed,
        box_size: [outfit.still_sprite.box_size, outfit.moving_sprite.box_size],
        boxes: [
            outfit.still_sprite.boxes.clone().try_into().unwrap(),
            outfit.moving_sprite.boxes.clone().try_into().unwrap(),
        ],
        phase_counts: [
            outfit.still_sprite.animation.total_animation_phases(),
            outfit.moving_sprite.animation.total_animation_phases(),
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
    outfit: &OutfitSprite,
    sheet: &SpriteSheet,
    materials: &mut Assets<ActorMaterial>,
    meshes: &mut Assets<Mesh>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    loaded_materials: &mut LoadedMaterials,
) {
    let params = ActorParams {
        atlas_grid: sheet.grid_size,
        pattern_x: UVec2::new(
            outfit.still_sprite.pattern_x,
            outfit.moving_sprite.pattern_x,
        ),
        pattern_y: UVec2::new(
            outfit.still_sprite.pattern_y,
            outfit.moving_sprite.pattern_y,
        ),
        pattern_z: UVec2::new(
            outfit.still_sprite.pattern_z,
            outfit.moving_sprite.pattern_z,
        ),
        layers: UVec2::new(outfit.still_sprite.layers, outfit.moving_sprite.layers),
    };

    let material_handle = materials.add(ActorMaterial {
        texture: sheet.texture.clone(),
        params,
        still_indexes: buffers.add(ShaderStorageBuffer::from(&outfit.still_sprite.sprite_ids)),
        moving_indexes: buffers.add(ShaderStorageBuffer::from(&outfit.moving_sprite.sprite_ids)),
        instances: loaded_materials.buffer.clone(),
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
    mut instances: ResMut<InstanceManager<ActorInstance>>,
    actor_q: Query<&MeshTag, With<Actor>>,
) {
    let Ok(tag) = actor_q.get(event.entity) else {
        return;
    };
    instances.dealloc_index(tag.0);
    commands.entity(event.entity).despawn();
}

pub fn upload_instance_buffer(
    instances: Res<InstanceManager<ActorInstance>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    loaded_materials: Res<LoadedMaterials>,
    mut materials: ResMut<Assets<ActorMaterial>>,
) {
    if !instances.is_changed() {
        return;
    }

    if let Some(ssb) = buffers.get_mut(&loaded_materials.buffer) {
        ssb.set_data(instances.get_buffer_data());
    }

    for (_, mat) in loaded_materials.materials.values() {
        // set material as changed so buffer gets updated in the pipeline
        let _ = materials.get_mut(mat).unwrap();
    }
}

pub fn update_actor_instances(
    actors_q: Query<(&Actor, &MeshTag, Option<&Moving>), Or<(Changed<Actor>, Changed<Moving>)>>,
    mut instances: ResMut<InstanceManager<ActorInstance>>,
) {
    for (actor, tag, moving) in actors_q {
        let index = tag.0;
        let instance = instances.get_mut(index);
        instance.moving = match moving {
            Some(..) => 1,
            None => 0,
        };
        instance.direction = actor.direction.into();
        instance.mounted = actor.mounted.into();
        instance.addons = actor.addons;
        instance.color_head = COLOR_TABLE[actor.color_head as usize];
        instance.color_body = COLOR_TABLE[actor.color_body as usize];
        instance.color_legs = COLOR_TABLE[actor.color_legs as usize];
        instance.color_feet = COLOR_TABLE[actor.color_feet as usize];
        instance.bounding_square = actor.box_size[instance.moving as usize];
        let bbox = &actor.boxes[instance.moving as usize][actor.direction as usize];
        instance.bbox_min = bbox.min;
        instance.bbox_size = bbox.max;
        instance.moving_progress = match moving {
            Some(m) => m.timer.fraction(),
            None => 0.0,
        };
        instance.phase_count = actor.phase_counts[instance.moving as usize];
    }
}

#[cfg(feature = "debug")]
pub fn actor_rect(actors_q: Query<(&Transform, &Actor, Option<&Moving>)>, mut gizmos: Gizmos) {
    for (pos, actor, moving) in &actors_q {
        gizmos.circle_2d(pos.translation.truncate(), 2.0, Color::srgb(1.0, 0.0, 0.0));

        let moving = if moving.is_some() { 1 } else { 0 } as usize;
        gizmos.rect_2d(
            pos.translation.truncate(),
            Vec2::splat(64.0),
            Color::srgb(0.0, 0.5, 1.0),
        );
        gizmos.rect_2d(
            pos.translation.truncate()
                + Vec2::new(
                    (64.0 - actor.box_size[moving]) / 4.0,
                    -((64.0 - actor.box_size[moving]) / 4.0),
                ),
            Vec2::splat(actor.box_size[moving]),
            Color::srgb(0.8, 0.5, 1.0),
        );

        let mesh_start = pos.translation.truncate();
        let iso =
            mesh_start + (actor.boxes[moving][actor.direction as usize].min * Vec2::new(0.5, -0.5));
        let bbox_size = actor.boxes[moving][actor.direction as usize].max;

        gizmos.rect_2d(iso, bbox_size, Color::srgb(1.0, 1.0, 0.0));
    }
}
