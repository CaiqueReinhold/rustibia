use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

use crate::actor::{spawn_actor, ActorInstance, ActorMaterial, LoadedMaterials};
use crate::conf::actor::{ADDON_1_FLAG, ADDON_2_FLAG};
use crate::core::{Appearances, GameState, InstanceManager};

use crate::network::events::SpawnPlayer;
use crate::player::components::Player;

pub fn check_game_ready(mut commands: Commands, player_q: Query<&Player>) {
    if !player_q.is_empty() {
        commands.set_state(GameState::InGame);
    }
}

pub fn spawn_player(
    event: On<SpawnPlayer>,
    mut commands: Commands,
    mut loaded_materials: ResMut<LoadedMaterials>,
    mut materials: ResMut<Assets<ActorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut instances: ResMut<InstanceManager<ActorInstance>>,
    appearances: Res<Appearances>,
    time: Res<Time>,
) {
    let entity = spawn_actor(
        &mut commands,
        &mut loaded_materials,
        &mut materials,
        &mut meshes,
        &mut buffers,
        &mut instances,
        &appearances,
        &time,
        event.outfit,
        0,
        0,
        0,
        0,
        272,
        ADDON_1_FLAG | ADDON_2_FLAG,
        event.position.clone(),
    );

    commands
        .entity(entity)
        .insert((Player, event.health.clone(), event.mana.clone()));
}
