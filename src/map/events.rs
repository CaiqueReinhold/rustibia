use bevy::prelude::*;

use crate::map::TilePosition;

#[derive(Event)]
pub struct TileChanged {
    pub position: TilePosition,
}
