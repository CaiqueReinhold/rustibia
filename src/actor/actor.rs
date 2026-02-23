use bevy::mesh::MeshTag;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

use crate::core::Appearances;

use crate::actor::assets::{Outfit, Outfits};
use crate::actor::colors::COLOR_TABLE;
use crate::actor::material::{ActorInstance, ActorMaterial, ActorParams};
use crate::actor::movement::Moving;

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
        for i in self.in_use.len()..=0 {
            if self.in_use[i] {
                return;
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

#[derive(Component, Debug)]
pub struct Actor {
    pub outfit_id: u32,
    pub direction: u32,
    pub addons: u32,
    pub mounted: u32,
    pub color_head: u32,
    pub color_body: u32,
    pub color_legs: u32,
    pub color_feet: u32,
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

pub fn on_spawn_actor(
    event: On<Add, Actor>,
    mut commands: Commands,
    mut loaded_materials: ResMut<LoadedMaterials>,
    mut materials: ResMut<Assets<ActorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut instances: ResMut<ActorInstances>,
    appearances: Res<Appearances>,
    outfits: Res<Outfits>,
    time: Res<Time>,
    actor_q: Query<&Actor>,
) {
    info!("Adding mesh to actor");
    let actor = actor_q.get(event.entity).unwrap();
    let outfit = outfits.outfits.get(&actor.outfit_id).unwrap();

    if !loaded_materials.materials.contains_key(&outfit.id) {
        init_material(
            outfit,
            &appearances,
            &mut materials,
            &mut meshes,
            &mut buffers,
            &mut loaded_materials,
            &instances,
        );
    }

    let (mesh, material) = loaded_materials.materials.get(&outfit.id).unwrap();
    let index = instances.alloc_index();
    instances.data[index as usize].time_offset = time.elapsed_secs_wrapped();
    commands.entity(event.entity).insert((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(material.clone()),
        MeshTag(index),
    ));
    info!("actor: {} mesh: {:?} material: {:?}", index, mesh, material);
}

fn init_material(
    outfit: &Outfit,
    appearances: &Appearances,
    materials: &mut Assets<ActorMaterial>,
    meshes: &mut Assets<Mesh>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    loaded_materials: &mut LoadedMaterials,
    instances: &ActorInstances,
) {
    let sheet = appearances.sheets.get(&outfit.sprite_group).unwrap();
    let still_sprite = appearances
        .sprite_configs
        .get(&outfit.still_sprite_id)
        .unwrap();
    let moving_sprite = appearances
        .sprite_configs
        .get(&outfit.moving_sprite_id)
        .unwrap();

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
        still_indexes: buffers.add(ShaderStorageBuffer::from(
            still_sprite.sprite_ids.as_slice(),
        )),
        moving_indexes: buffers.add(ShaderStorageBuffer::from(
            still_sprite.sprite_ids.as_slice(),
        )),
        instances: instances.buffer.clone(),
    });

    let mesh = Mesh::from(Rectangle::new(
        still_sprite.box_size.x as f32,
        still_sprite.box_size.y as f32,
    ));
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
) {
    if !instances.is_changed() {
        return;
    }

    if let Some(ssb) = buffers.get_mut(&instances.buffer) {
        ssb.set_data(instances.data.as_slice());
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
        instance.direction = actor.direction;
        instance.mounted = actor.mounted;
        instance.addons = actor.addons;
        instance.color_head = COLOR_TABLE[actor.color_head as usize];
        instance.color_body = COLOR_TABLE[actor.color_body as usize];
        instance.color_legs = COLOR_TABLE[actor.color_legs as usize];
        instance.color_feet = COLOR_TABLE[actor.color_feet as usize];
    }
}
