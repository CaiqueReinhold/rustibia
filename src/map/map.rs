use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::map::position::TilePosition;

pub struct TileConfig {
    pub id: u32,
    pub sprite_id: u32,
    pub ground_speed: u32,
    pub can_walk: bool,
    pub fullbank: bool,
    pub avoid: bool,
    pub minimap_color: u32,
}
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
