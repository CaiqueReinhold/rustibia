use bevy::prelude::*;

use crate::actor::{actor::Actor, hud::Health};
use crate::camera::GameCamera;
use crate::map::TilePosition;

use crate::conf::z_order::ACTOR_Z_OFFSET;

#[derive(Component)]
pub struct Player {
    pub max_experience: u32,
    pub experience: u32,
    // pub speed: u32,
}

pub fn spawn_player(mut commands: Commands) {
    info!("Player spawned");
    let position = TilePosition {
        x: 1028,
        y: 1028,
        floor: 7,
    };
    let world_position = position.to_world();
    info!("player position: {}", world_position);
    commands.spawn((
        Player {
            max_experience: 100,
            experience: 0,
        },
        Health {
            current: 150,
            max: 150,
        },
        Actor {
            outfit_id: 1649,
            direction: 0,
            addons: 0,
            mounted: 0,
            color_head: 0,
            color_body: 0,
            color_feet: 0,
            color_legs: 0,
        },
        position,
        Transform::from_xyz(
            world_position.x,
            world_position.y,
            world_position.z + ACTOR_Z_OFFSET,
        ),
    ));
}

pub fn center_on_player(
    player_q: Single<&Transform, With<Player>>,
    camera_q: Single<&mut Transform, (With<GameCamera>, Without<Player>)>,
) {
    let player_transform = *player_q;
    let mut camera_transform = camera_q;

    camera_transform.translation = player_transform.translation;
}
