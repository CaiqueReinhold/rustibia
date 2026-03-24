use bevy::prelude::*;

use std::ops::{Add, Sub};

use crate::actor::WalkingDirection;
use crate::conf::map::TILE_SIZE;
use crate::conf::z_order::{FLOOR_Z_MULTIPLIER, POSITION_Z_MULTIPLIER};

#[derive(Component, Hash, PartialEq, Eq, Clone, Debug)]
pub struct TilePosition {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl TilePosition {
    // pub fn new(x: u32, y: u32, z: u32) -> Self {
    //     TilePosition { x, y, z }
    // }

    pub fn from_world(world_pos: Vec2, z: u32) -> Self {
        TilePosition {
            x: (world_pos.x / TILE_SIZE).floor() as u32,
            y: (world_pos.y.abs() / TILE_SIZE).floor() as u32,
            z,
        }
    }

    pub fn to_world(&self) -> Vec3 {
        Vec3::new(
            (self.x as f32) * TILE_SIZE,
            -(self.y as f32) * TILE_SIZE,
            (self.z as f32 * FLOOR_Z_MULTIPLIER)
                + ((self.x as f32) + (self.y as f32)) * POSITION_Z_MULTIPLIER,
        )
    }

    pub fn delta(&self, x: i32, y: i32) -> Self {
        TilePosition {
            x: ((self.x as i32) + x) as u32,
            y: ((self.y as i32) + y) as u32,
            z: self.z,
        }
    }
}

impl Add<WalkingDirection> for TilePosition {
    type Output = TilePosition;

    fn add(self, rhs: WalkingDirection) -> Self::Output {
        match rhs {
            WalkingDirection::North => self.delta(0, -1),
            WalkingDirection::East => self.delta(1, 0),
            WalkingDirection::South => self.delta(0, 1),
            WalkingDirection::West => self.delta(-1, 0),
            WalkingDirection::NorthEast => self.delta(1, -1),
            WalkingDirection::SouthEast => self.delta(1, 1),
            WalkingDirection::NorthWest => self.delta(-1, -1),
            WalkingDirection::SouthWest => self.delta(-1, 1),
        }
    }
}

impl Sub<WalkingDirection> for TilePosition {
    type Output = TilePosition;

    fn sub(self, rhs: WalkingDirection) -> Self::Output {
        match rhs {
            WalkingDirection::North => self.delta(0, 1),
            WalkingDirection::East => self.delta(-1, 0),
            WalkingDirection::South => self.delta(0, -1),
            WalkingDirection::West => self.delta(1, 0),
            WalkingDirection::NorthEast => self.delta(-1, 1),
            WalkingDirection::SouthEast => self.delta(-1, -1),
            WalkingDirection::NorthWest => self.delta(1, 1),
            WalkingDirection::SouthWest => self.delta(1, -1),
        }
    }
}

// #[derive(Component, Debug, Hash, Eq, PartialEq, Clone, Default)]
// pub struct ChunkPosition {
//     pub cx: u32,
//     pub cy: u32,
//     pub z: u32,
// }

// impl ChunkPosition {
//     pub fn new(cx: u32, cy: u32, z: u32) -> Self {
//         ChunkPosition { cx, cy, z }
//     }

//     pub fn from_tile(tile_pos: &TilePosition) -> Self {
//         Self {
//             cx: tile_pos.x / CHUNK_SIZE,
//             cy: tile_pos.y / CHUNK_SIZE,
//             z: tile_pos.z,
//         }
//     }

//     pub fn start_position(&self) -> TilePosition {
//         TilePosition {
//             x: self.cx * CHUNK_SIZE,
//             y: self.cy * CHUNK_SIZE,
//             z: self.z,
//         }
//     }
// }
