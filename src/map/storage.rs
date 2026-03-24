use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use thiserror::Error;

use crate::items::{Item, ItemFlag};
use crate::map::position::TilePosition;

#[derive(Error, Debug)]
pub enum MapOperationError {
    #[error("Item cannot be moved")]
    CannotMoveItem,
}

#[derive(Debug)]
pub struct MapTile {
    pub items: Vec<Arc<Item>>,
}

#[derive(Resource, Default)]
pub struct Map {
    tiles: HashMap<TilePosition, MapTile>,
}

impl Map {
    pub fn replace_tile(&mut self, items: Vec<Arc<Item>>, pos: &TilePosition) {
        self.tiles.insert(pos.clone(), MapTile { items });
    }

    pub fn can_walk(&self, pos: &TilePosition) -> bool {
        let tile = match self.tiles.get(pos) {
            Some(t) => t,
            None => return false,
        };

        let has_ground = tile
            .items
            .iter()
            .any(|i| i.config.has_flag(ItemFlag::Ground));
        if !has_ground {
            return false;
        }

        let blocked = tile
            .items
            .iter()
            .any(|i| i.config.has_flag(ItemFlag::Unpass));
        !blocked
    }

    fn can_move(&self, pos: &TilePosition) -> bool {
        let tile = match self.tiles.get(pos) {
            Some(t) => t,
            None => return false,
        };

        !tile
            .items
            .iter()
            .any(|i| i.config.has_flag(ItemFlag::FullBank))
    }

    pub fn add_item(
        &mut self,
        item: Arc<Item>,
        position: &TilePosition,
    ) -> Result<(), MapOperationError> {
        if !self.can_move(position) {
            return Err(MapOperationError::CannotMoveItem);
        }

        if item.config.has_flag(ItemFlag::Unmove) {
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

    pub fn get_tile_friction(&self, pos: &TilePosition) -> u8 {
        let tile = match self.tiles.get(pos) {
            Some(t) => t,
            None => return 100,
        };

        tile.items
            .iter()
            .find(|i| i.config.has_flag(ItemFlag::Ground))
            .and_then(|i| i.config.friction)
            .unwrap_or_default()
    }

    pub fn get_items(&self, pos: &TilePosition) -> Option<impl Iterator<Item = &Item>> {
        let tile = self.tiles.get(pos)?;
        Some(tile.items.iter().map(|i| i.as_ref()))
    }
}

pub(super) fn init_map(mut commands: Commands) {
    commands.init_resource::<Map>();
}
