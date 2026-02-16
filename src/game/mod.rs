pub mod actor;
pub mod hud;
pub mod map;
pub mod player;

use bevy::prelude::*;

use crate::conf::map::{FLOOR_Z_OFFSET, TILE_SIZE};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(actor::ActorPlugin)
            .add_plugins(map::MapPlugin);
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
    pub x: f32,
    pub y: f32,
    pub z: u32,
}

impl TilePosition {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        TilePosition {
            x: x as f32,
            y: y as f32,
            z: z,
        }
    }

    pub fn to_world_position(&self) -> Vec3 {
        Vec3::new(
            self.x * TILE_SIZE,
            -self.y * TILE_SIZE,
            (self.z as f32 * FLOOR_Z_OFFSET) + 0.1,
        )
    }

    pub fn absolute(&self) -> UVec3 {
        UVec3 {
            x: self.x as u32,
            y: self.y as u32,
            z: self.z,
        }
    }
}
