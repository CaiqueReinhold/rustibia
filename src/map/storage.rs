use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use thiserror::Error;

use crate::items::{Item, ItemConfig};
use crate::map::position::TilePosition;

#[derive(Error, Debug)]
pub enum MapOperationError {
    #[error("Item cannot be moved")]
    CannotMoveItem,
}

#[derive(Debug)]
pub struct MapTile {
    pub ground: Option<Arc<ItemConfig>>,
    pub border: Option<Arc<ItemConfig>>,
    pub items: Vec<Arc<Item>>,
}

#[derive(Resource)]
pub struct Map {
    // pub width: u32,
    // pub height: u32,
    pub floors: u32,
    pub tiles: HashMap<TilePosition, MapTile>,
}

impl Map {
    pub fn can_walk(&self, pos: &TilePosition) -> bool {
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

    fn can_move(&self, pos: &TilePosition) -> bool {
        let tile = match self.tiles.get(pos) {
            Some(t) => t,
            None => return false,
        };

        if let Some(ground) = &tile.ground {
            if ground.have_fullbank {
                return false;
            }
        }

        if let Some(border) = &tile.border {
            if border.have_fullbank {
                return false;
            }
        }

        true
    }

    pub fn add_item(
        &mut self,
        item: Arc<Item>,
        position: &TilePosition,
    ) -> Result<(), MapOperationError> {
        if !self.can_move(position) {
            return Err(MapOperationError::CannotMoveItem);
        }

        if !item.config.can_move {
            return Err(MapOperationError::CannotMoveItem);
        }

        let Some(tile) = self.tiles.get_mut(position) else {
            return Err(MapOperationError::CannotMoveItem);
        };
        tile.items.push(item);
        Ok(())
    }

    pub fn remove_item(
        &mut self,
        index: usize,
        position: &TilePosition,
    ) -> Result<(), MapOperationError> {
        let Some(tile) = self.tiles.get_mut(position) else {
            return Err(MapOperationError::CannotMoveItem);
        };
        tile.items.remove(index);
        Result::Ok(())
    }

    pub fn peek_item(&self, position: &TilePosition) -> Option<(&Arc<Item>, usize)> {
        let tile = self.tiles.get(position)?;
        let item = tile.items.last()?;
        let index = tile.items.len() - 1;
        Some((item, index))
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
