use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::map::position::TilePosition;

#[derive(Debug)]
pub struct TileConfig {
    pub id: u32,
    pub sprite_id: u32,
    pub ground_speed: u32,
    pub can_walk: bool,
    pub fullbank: bool,
    pub avoid: bool,
    pub minimap_color: u32,
}

#[derive(Debug)]
pub struct MapTile {
    pub ground: Option<Arc<TileConfig>>,
    pub border: Option<Arc<TileConfig>>,
}

#[derive(Resource)]
pub struct Map {
    pub width: u32,
    pub height: u32,
    pub floors: u32,
    pub tiles: HashMap<TilePosition, MapTile>,
}

impl Map {
    pub fn can_move(&self, pos: &TilePosition) -> bool {
        let tile = match self.tiles.get(pos) {
            Some(t) => t,
            None => return false,
        };

        if let Some(ground) = &tile.ground {
            if !ground.can_walk {
                return false;
            }
        }

        if let Some(border) = &tile.border {
            if !border.can_walk {
                return false;
            }
        }

        true
    }

    pub fn get_step_duration_ms(
        &self,
        pos: &TilePosition,
        base_speed: u32,
        is_diagonal: bool,
    ) -> u32 {
        if base_speed == 0 {
            return 1000;
        }

        let tile = match self.tiles.get(pos) {
            Some(t) => t,
            None => return 1000,
        };

        let ground_speed = tile
            .ground
            .as_ref()
            .map(|g| g.ground_speed)
            .filter(|s| *s > 0)
            .unwrap_or(100);

        let mut step = ((1000u32).saturating_mul(ground_speed) / base_speed).max(1);

        if is_diagonal {
            step = step.saturating_mul(3) / 2;
        }

        step.max(1)
    }
}
