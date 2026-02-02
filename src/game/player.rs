use bevy::{prelude::*, sprite::Anchor};

use crate::game::hud::{Health, Mana};
use crate::ui::gameview::GameCamera;

const TILE_SIZE: f32 = 32.0;
const SOUTH_ANIMATION_INDEXES: [usize; 3] = [0, 1, 2];
const EAST_ANIMATION_INDEXES: [usize; 3] = [3, 4, 5];
const NORTH_ANIMATION_INDEXES: [usize; 3] = [6, 7, 8];
const WEST_ANIMATION_INDEXES: [usize; 3] = [9, 10, 11];

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player).add_systems(
            Update,
            (
                camera_follow_player,
                tile_movement_input,
                execute_tile_movement,
                animate_player,
            ),
        );
    }
}

#[derive(Component)]
pub struct Player {
    pub max_experience: u32,
    pub experience: u32,
    // pub level: u32,
    pub speed: f32,
}

#[derive(Component)]
pub struct TilePosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct MoveTo {
    pub start: Vec3,
    pub end: Vec3,
    pub timer: Timer,
}

#[derive(Component)]
pub struct Facing(pub Direction);

#[derive(Clone, Copy)]
pub enum Direction {
    North,
    South,
    West,
    East,
}

fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture_handle = asset_server.load("sprites/player.png");
    let texture_atlas = TextureAtlasLayout::from_grid(
        UVec2 { x: 40, y: 40 },
        1,
        12,
        Some(UVec2 { x: 0, y: 24 }),
        Some(UVec2::splat(0)),
    );
    let texture_atlas_handle = atlases.add(texture_atlas);
    let atlas = TextureAtlas::from(texture_atlas_handle);

    let tile_x = 0;
    let tile_y = 0;

    let world_pos = Vec3::new(
        tile_x as f32 * TILE_SIZE + TILE_SIZE / 2.0,
        tile_y as f32 * TILE_SIZE + TILE_SIZE / 2.0,
        10.0,
    );

    commands.spawn((
        Sprite {
            image: texture_handle.clone(),
            texture_atlas: Some(atlas),
            ..default()
        },
        Anchor(Vec2 { x: 0.3, y: 0.4 }),
        Transform::from_xyz(world_pos.x, world_pos.y, world_pos.z),
        Player {
            max_experience: 100,
            experience: 0,
            // level: 1,
            speed: 200.0,
        },
        Health {
            max: 100,
            current: 100,
        },
        Mana {
            max: 100,
            current: 100,
        },
        // DisplayName {
        //     name: "Rizael".to_string(),
        // },
        TilePosition {
            x: tile_x,
            y: tile_y,
        },
        Facing(Direction::South),
        Name::new("Player"),
    ));
}

fn tile_movement_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    query: Query<(Entity, &TilePosition, &Transform, &Player), (With<Player>, Without<MoveTo>)>,
) {
    let Ok((entity, tile, transform, player)) = query.single() else {
        return;
    };

    let (dx, dy, facing) = if keyboard.just_pressed(KeyCode::KeyW) {
        (0, 1, Direction::North)
    } else if keyboard.just_pressed(KeyCode::KeyS) {
        (0, -1, Direction::South)
    } else if keyboard.just_pressed(KeyCode::KeyA) {
        (-1, 0, Direction::West)
    } else if keyboard.just_pressed(KeyCode::KeyD) {
        (1, 0, Direction::East)
    } else {
        return;
    };

    let start = transform.translation;
    let end = Vec3::new(
        (tile.x + dx) as f32 * TILE_SIZE + TILE_SIZE / 2.0,
        (tile.y + dy) as f32 * TILE_SIZE + TILE_SIZE / 2.0,
        start.z,
    );

    commands.entity(entity).insert((
        MoveTo {
            start,
            end,
            timer: Timer::from_seconds(player.speed / 400.0, TimerMode::Once),
        },
        Facing(facing),
    ));
}

fn execute_tile_movement(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut TilePosition, &mut MoveTo), With<Player>>,
) {
    let Ok((entity, mut transform, mut tile, mut move_to)) = query.single_mut() else {
        return;
    };

    move_to.timer.tick(time.delta());

    let t = move_to.timer.fraction();
    transform.translation = move_to.start.lerp(move_to.end, t);

    if move_to.timer.is_finished() {
        transform.translation = move_to.end;

        let dx = ((move_to.end.x - move_to.start.x) / TILE_SIZE).round() as i32;
        let dy = ((move_to.end.y - move_to.start.y) / TILE_SIZE).round() as i32;

        tile.x += dx;
        tile.y += dy;

        commands.entity(entity).remove::<MoveTo>();
    }
}

fn animate_player(mut query: Query<(&MoveTo, &Facing, &mut Sprite), With<Player>>) {
    if let Ok((move_to, facing, mut sprite)) = query.single_mut() {
        if let Some(atlas) = &mut sprite.texture_atlas {
            let fraction = move_to.timer.fraction();

            match facing.0 {
                Direction::North => {
                    let indexes = NORTH_ANIMATION_INDEXES;
                    let frame = (fraction * indexes.len() as f32).floor() as usize % indexes.len();
                    atlas.index = indexes[frame];
                }
                Direction::South => {
                    let indexes = SOUTH_ANIMATION_INDEXES;
                    let frame = (fraction * indexes.len() as f32).floor() as usize % indexes.len();
                    atlas.index = indexes[frame];
                }
                Direction::East => {
                    let indexes = EAST_ANIMATION_INDEXES;
                    let frame = (fraction * indexes.len() as f32).floor() as usize % indexes.len();
                    atlas.index = indexes[frame];
                }
                Direction::West => {
                    let indexes = WEST_ANIMATION_INDEXES;
                    let frame = (fraction * indexes.len() as f32).floor() as usize % indexes.len();
                    atlas.index = indexes[frame];
                }
            }
        }
    }
}

fn camera_follow_player(
    player: Query<&Transform, With<Player>>,
    mut camera: Query<&mut Transform, (With<GameCamera>, Without<Player>)>,
) {
    let player_transform = player.single().unwrap();
    let mut camera_transform = camera.single_mut().unwrap();

    camera_transform.translation.x = player_transform.translation.x;
    camera_transform.translation.y = player_transform.translation.y;
}
