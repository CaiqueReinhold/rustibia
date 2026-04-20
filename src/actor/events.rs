use bevy::{prelude::*, render::storage::ShaderStorageBuffer};

use crate::{
    actor::{spawn_actor, ActorInstance, ActorMaterial, LoadedMaterials},
    conf::actor::{ADDON_1_FLAG, ADDON_2_FLAG},
    core::{Appearances, InstanceManager},
    game_ui::GameUiAssets,
    map::Map,
    network::events::SpawnAgent,
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
        event.outfit.1,
        event.facing,
        event.speed,
        ADDON_1_FLAG | ADDON_2_FLAG,
        event.position.clone(),
        event.name.clone(),
        Some(event.health.clone()),
        None,
    );
    map.add_agent(event.agent_id, entity);
}
