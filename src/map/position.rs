use bevy::prelude::*;

use crate::conf::map::{CHUNK_SIZE, TILE_SIZE};
use crate::conf::z_order::FLOOR_Z_MULTIPLIER;

#[derive(Component, Hash, PartialEq, Eq, Clone)]
pub struct TilePosition {
    pub x: u32,
    pub y: u32,
    pub floor: u32,
}

impl TilePosition {
    pub fn new(x: u32, y: u32, floor: u32) -> Self {
        TilePosition { x, y, floor }
    }

    pub fn to_world(&self) -> Vec3 {
        Vec3::new(
            (self.x as f32) * TILE_SIZE,
            -(self.y as f32) * TILE_SIZE,
            self.floor as f32 * FLOOR_Z_MULTIPLIER,
        )
    }
}

#[derive(Component, Debug, Hash, Eq, PartialEq, Clone, Default)]
pub struct ChunkPosition {
    pub cx: u32,
    pub cy: u32,
    pub floor: u32,
}

impl ChunkPosition {
    pub fn new(cx: u32, cy: u32, floor: u32) -> Self {
        ChunkPosition { cx, cy, floor }
    }

    pub fn from_tile(tile_pos: &TilePosition) -> Self {
        Self {
            cx: tile_pos.x / CHUNK_SIZE,
            cy: tile_pos.y / CHUNK_SIZE,
            floor: tile_pos.floor,
        }
    }

    pub fn start_position(&self) -> TilePosition {
        TilePosition {
            x: self.cx * CHUNK_SIZE,
            y: self.cy * CHUNK_SIZE,
            floor: self.floor,
        }
    }
}
