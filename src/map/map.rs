use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::items::{Item, ItemConfig};
use crate::map::position::TilePosition;

#[derive(Debug)]
pub struct MapTile {
    pub ground: Option<Arc<ItemConfig>>,
    pub border: Option<Arc<ItemConfig>>,
    pub items: Vec<Item>,
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

        for item in tile.items.iter() {
            if !item.config.can_walk {
                return false;
            }
        }

        true
    }

    pub fn get_tile_speed_modifier(&self, pos: &TilePosition) -> u32 {
        let tile = match self.tiles.get(pos) {
            Some(t) => t,
            None => return 100,
        };

        let ground_speed = tile
            .ground
            .as_ref()
            .map(|g| g.ground_speed)
            .filter(|s| *s > 0)
            .unwrap_or(100);

        ground_speed
    }
}
