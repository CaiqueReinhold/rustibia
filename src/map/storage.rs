use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::actor::AgentId;
use crate::items::{Item, ItemFlag};
use crate::map::position::Position;

#[derive(Debug)]
pub struct MapTile {
    pub items: Vec<Arc<Item>>,
}

#[derive(Resource, Default)]
pub struct Map {
    tiles: HashMap<Position, MapTile>,
    agents: HashMap<AgentId, Entity>,
}

impl Map {
    pub fn add_agent(&mut self, id: AgentId, agent: Entity) {
        self.agents.insert(id, agent);
    }

    pub fn get_agent(&self, id: AgentId) -> Option<Entity> {
        self.agents.get(&id).cloned()
    }

    pub fn replace_tile(&mut self, items: Vec<Arc<Item>>, pos: &Position) {
        self.tiles.insert(pos.clone(), MapTile { items });
    }

    pub fn can_walk(&self, pos: &Position) -> bool {
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

    pub fn can_drop_item(&self, pos: &Position) -> bool {
        let tile = match self.tiles.get(pos) {
            Some(t) => t,
            None => return false,
        };

        tile.items
            .iter()
            .any(|i| i.config.has_flag(ItemFlag::FullBank))
            && !tile
                .items
                .iter()
                .any(|i| i.config.has_flag(ItemFlag::Bottom))
    }

    pub fn peek_item(&self, position: &Position) -> Option<(&Arc<Item>, usize)> {
        let tile = self.tiles.get(position)?;
        let item = tile.items.last()?;
        let index = tile.items.len() - 1;
        Some((item, index))
    }

    pub fn get_tile_friction(&self, pos: &Position) -> Option<u8> {
        let tile = self.tiles.get(pos)?;

        if !self.can_walk(pos) {
            return None;
        }

        tile.items
            .iter()
            .find(|i| i.config.has_flag(ItemFlag::Ground))
            .and_then(|i| i.config.friction)
    }

    pub fn get_items(&self, pos: &Position) -> Option<impl Iterator<Item = &Item>> {
        let tile = self.tiles.get(pos)?;
        Some(tile.items.iter().map(|i| i.as_ref()))
    }

    pub fn get_minimap_color(&self, pos: &Position) -> Option<u8> {
        let tile = self.tiles.get(pos)?;
        tile.items
            .iter()
            .rev()
            .find_map(|it| it.config.minimap_color)
    }

    pub fn avoid(&self, pos: &Position) -> bool {
        let Some(tile) = self.tiles.get(pos) else {
            return true;
        };

        tile.items
            .iter()
            .any(|it| it.config.has_flag(ItemFlag::Avoid))
    }

    pub fn get_elevation(&self, pos: &Position) -> u8 {
        let Some(tile) = self.tiles.get(pos) else {
            return 0;
        };

        tile.items
            .iter()
            .filter_map(|it| it.config.elevation)
            .take(3)
            .sum()
    }

    pub fn is_ground(&self, pos: &Position) -> bool {
        let Some(tile) = self.tiles.get(pos) else {
            return false;
        };
        tile.items
            .iter()
            .any(|it| it.config.has_flag(ItemFlag::Ground) || it.config.has_flag(ItemFlag::Border))
    }

    pub fn is_bottom(&self, pos: &Position) -> bool {
        let Some(tile) = self.tiles.get(pos) else {
            return false;
        };
        tile.items
            .iter()
            .any(|it| it.config.has_flag(ItemFlag::Bottom))
    }

    pub fn block_sight(&self, pos: &Position) -> bool {
        let Some(tile) = self.tiles.get(pos) else {
            return false;
        };
        tile.items
            .iter()
            .any(|it| it.config.has_flag(ItemFlag::BlockSight))
    }
}

pub(super) fn init_map(mut commands: Commands) {
    commands.init_resource::<Map>();
}
