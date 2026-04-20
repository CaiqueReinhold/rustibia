use bevy::{mesh::MeshTag, prelude::*, render::storage::ShaderStorageBuffer};

use crate::{
    actor::{
        spawn_actor, Actor, ActorHud, ActorInstance, ActorMaterial, LoadedMaterials, MoveActor,
        MoveQueue, Moving,
    },
    conf::actor::{ADDON_1_FLAG, ADDON_2_FLAG},
    core::{Appearances, InstanceManager},
    game_ui::GameUiAssets,
    map::{Map, Position},
    network::events::{MoveAgent, RemoveAgent, SpawnAgent},
};

pub fn on_spawn_agent(
    event: On<SpawnAgent>,
    mut commands: Commands,
    mut loaded_materials: ResMut<LoadedMaterials>,
    mut materials: ResMut<Assets<ActorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut instances: ResMut<InstanceManager<ActorInstance>>,
    mut map: ResMut<Map>,
    ui_assets: Res<GameUiAssets>,
    appearances: Res<Appearances>,
) {
    if let Some(entity) = map.get_agent(event.agent_id) {
        commands.entity(entity).despawn();
        map.remove_agent(event.agent_id);
    }

    let entity = spawn_actor(
        &mut commands,
        &mut loaded_materials,
        &mut materials,
        &mut meshes,
        &mut buffers,
        &mut instances,
        &ui_assets.font,
        &appearances,
        event.outfit.0,
        &map,
        event.outfit.1,
        event.facing,
        event.speed,
        ADDON_1_FLAG | ADDON_2_FLAG,
        event.position.clone(),
        event.name.clone(),
        Some(event.health.clone()),
        None,
        event.agent_id,
    );
    map.add_agent(event.agent_id, entity);
}

pub fn on_move_agent(
    event: On<MoveAgent>,
    mut commands: Commands,
    map: Res<Map>,
    pos_q: Query<&Position>,
    mut queue_q: Query<&mut MoveQueue>,
    moving_q: Query<&Moving>,
) {
    let Some(agent_entity) = map.get_agent(event.agent_id) else {
        return;
    };

    if moving_q.get(agent_entity).is_ok() {
        if let Ok(mut queue) = queue_q.get_mut(agent_entity) {
            queue.0.push_back((event.from.clone(), event.direction));
        }
        return;
    }

    let Ok(pos) = pos_q.get(agent_entity) else {
        return;
    };
    if *pos != event.from {
        commands.entity(agent_entity).insert(event.from.clone());
    }

    commands.trigger(MoveActor {
        agent_id: event.agent_id,
        direction: event.direction,
    });
}

pub fn on_remove_actor(
    event: On<RemoveAgent>,
    mut commands: Commands,
    mut instances: ResMut<InstanceManager<ActorInstance>>,
    actor_q: Query<(&MeshTag, Option<&ActorHud>), With<Actor>>,
    mut map: ResMut<Map>,
) {
    let Some(agent_entity) = map.get_agent(event.agent_id) else {
        return;
    };
    let Ok((tag, maybe_hud)) = actor_q.get(agent_entity) else {
        return;
    };
    if let Some(hud) = maybe_hud {
        commands.entity(hud.main_entity).despawn();
    }
    instances.dealloc_index(tag.0);
    commands.entity(agent_entity).despawn();
    map.remove_agent(event.agent_id);
}
