use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::agent::AgentId;
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

    pub fn remove_agent(&mut self, id: AgentId) {
        self.agents.remove(&id);
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

    pub fn item_at(&self, position: &Position, index: usize) -> Option<&Arc<Item>> {
        self.tiles.get(position)?.items.get(index)
    }

    pub fn get_tile_friction(&self, pos: &Position) -> Option<u16> {
        let tile = self.tiles.get(pos)?;

        if !self.can_walk(pos) {
            return None;
        }

        // The server takes the first item carrying a `tile_friction` attribute
        // (`GameMap::tile_friction`). Matching that rule rather than filtering on
        // the Ground flag separately keeps the two sides from drifting apart; the
        // parse in `core::items` is what makes the two selections equivalent.
        tile.items.iter().find_map(|i| i.config.friction)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{ItemConfig, ItemFlag, ItemId};

    fn item(id: ItemId, flags: Vec<ItemFlag>, friction: Option<u16>) -> Arc<Item> {
        Arc::new(Item::new(
            Arc::new(ItemConfig {
                id,
                flags,
                friction,
                slot: None,
                minimap_color: None,
                elevation: None,
            }),
            1,
        ))
    }

    fn at(x: u16, y: u16) -> Position {
        Position { x, y, z: 7 }
    }

    /// The rule is the server's: first item carrying a value, not first item
    /// carrying the Ground flag. Items stacked above the ground have no friction,
    /// so the two agree — this pins that they keep agreeing.
    #[test]
    fn friction_comes_from_the_first_item_that_has_it() {
        let mut map = Map::default();
        map.replace_tile(
            vec![
                item(100, vec![ItemFlag::Ground], Some(150)),
                item(200, Vec::new(), None),
            ],
            &at(10, 10),
        );

        assert_eq!(map.get_tile_friction(&at(10, 10)), Some(150));
    }

    /// The regression this task exists for: 260 used to truncate through a `u8`
    /// into 4, predicting a 50ms step where the server charges 900ms.
    #[test]
    fn friction_above_255_survives() {
        let mut map = Map::default();
        map.replace_tile(
            vec![item(21718, vec![ItemFlag::Ground], Some(260))],
            &at(10, 10),
        );

        assert_eq!(map.get_tile_friction(&at(10, 10)), Some(260));
    }

    /// An unwalkable tile has no friction to report, which is what keeps the
    /// minimap's A* from routing through it.
    #[test]
    fn an_unwalkable_tile_reports_no_friction() {
        let mut map = Map::default();
        map.replace_tile(
            vec![
                item(100, vec![ItemFlag::Ground], Some(150)),
                item(200, vec![ItemFlag::Unpass], None),
            ],
            &at(10, 10),
        );

        assert_eq!(map.get_tile_friction(&at(10, 10)), None);
    }

    #[test]
    fn an_unknown_tile_reports_no_friction() {
        let map = Map::default();
        assert_eq!(map.get_tile_friction(&at(10, 10)), None);
    }
}
