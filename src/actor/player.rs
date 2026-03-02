use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

use crate::actor::actions::{PlayerChangeDirection, PlayerMove};
use crate::actor::actor::{ActorInstances, FacingDirection, LoadedMaterials};
use crate::actor::material::ActorMaterial;
use crate::actor::movement::WalkingDirection;
use crate::actor::{actor::spawn_actor, hud::Health};
use crate::actor::{Mana, Outfits};
use crate::camera::GameCamera;
use crate::conf::map::TILE_SIZE;
use crate::core::Appearances;
use crate::map::TilePosition;

#[derive(Component)]
pub struct Player {
    pub max_experience: u32,
    pub experience: u32,
}

pub fn spawn_player(
    mut commands: Commands,
    mut loaded_materials: ResMut<LoadedMaterials>,
    mut materials: ResMut<Assets<ActorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut instances: ResMut<ActorInstances>,
    appearances: Res<Appearances>,
    outfits: Res<Outfits>,
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
        &outfits,
        &time,
        1649,
        0,
        0,
        0,
        0,
        500,
        TilePosition {
            x: 1028,
            y: 1028,
            floor: 7,
        },
    );

    use bevy::sprite::Text2dShadow;
    commands.entity(entity).insert((
        Player {
            max_experience: 100,
            experience: 0,
        },
        Health {
            current: 150,
            max: 150,
        },
        Mana {
            current: 100,
            max: 120,
        },
        children![(
            Text2d::new("1028, 1028"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            Transform::from_translation(Vec3 {
                x: 0.0,
                y: 44.0,
                z: 0.0
            }),
            Text2dShadow {
                offset: Vec2::new(1.2, 1.2),
                color: Color::BLACK
            }
        )],
    ));
}

pub fn center_on_player(
    player_q: Single<&Transform, With<Player>>,
    camera_q: Single<&mut Transform, (With<GameCamera>, Without<Player>)>,
) {
    let player_transform = *player_q;
    let mut camera_transform = camera_q;

    camera_transform.translation =
        player_transform.translation + Vec3::new(-(TILE_SIZE / 2.0), -(TILE_SIZE / 2.0), 0.0);
}

pub fn read_player_input(keyboard: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyW) {
        commands.trigger(PlayerChangeDirection {
            direction: FacingDirection::North,
        });
    } else if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyD) {
        commands.trigger(PlayerChangeDirection {
            direction: FacingDirection::East,
        })
    } else if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyS) {
        commands.trigger(PlayerChangeDirection {
            direction: FacingDirection::South,
        })
    } else if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyA) {
        commands.trigger(PlayerChangeDirection {
            direction: FacingDirection::West,
        })
    } else if keyboard.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::North,
        });
    } else if keyboard.any_just_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::East,
        });
    } else if keyboard.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::South,
        });
    } else if keyboard.any_just_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::West,
        });
    } else if keyboard.just_pressed(KeyCode::KeyQ) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::NorthWest,
        });
    } else if keyboard.just_pressed(KeyCode::KeyE) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::NorthEast,
        });
    } else if keyboard.just_pressed(KeyCode::KeyZ) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::SouthWest,
        });
    } else if keyboard.just_pressed(KeyCode::KeyC) {
        commands.trigger(PlayerMove {
            direction: WalkingDirection::SouthEast,
        });
    }
}

pub fn show_pos(
    pos_q: Single<&TilePosition, (With<Player>, Changed<TilePosition>)>,
    mut text_q: Single<&mut Text2d>,
) {
    let posx = pos_q.x;
    let posy = pos_q.y;

    text_q.0 = format!("{}, {}", posx, posy);
}
