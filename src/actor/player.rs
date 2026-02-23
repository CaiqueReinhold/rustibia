use bevy::prelude::*;

use crate::actor::actions::PlayerMove;
use crate::actor::actor::{FacingDirection, Mounted};
use crate::actor::movement::WalkingDirection;
use crate::actor::{actor::Actor, hud::Health};
use crate::camera::GameCamera;
use crate::conf::actor::ADDONS_NONE;
use crate::map::TilePosition;

use crate::conf::z_order::ACTOR_Z_OFFSET;

#[derive(Component)]
pub struct Player {
    pub max_experience: u32,
    pub experience: u32,
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
            direction: FacingDirection::North,
            addons: ADDONS_NONE,
            mounted: Mounted::Unmounted,
            color_head: 0,
            color_body: 0,
            color_feet: 0,
            color_legs: 0,
            speed: 500,
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

pub fn read_player_input(keyboard: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keyboard.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::North,
        });
    } else if keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::East,
        });
    } else if keyboard.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::South,
        });
    } else if keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::West,
        });
    } else if keyboard.pressed(KeyCode::KeyQ) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::NorthWest,
        });
    } else if keyboard.pressed(KeyCode::KeyE) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::NorthEast,
        });
    } else if keyboard.pressed(KeyCode::KeyZ) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::SouthWest,
        });
    } else if keyboard.pressed(KeyCode::KeyC) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::SouthEast,
        });
    }
}
