use std::sync::Arc;

use crate::{core::SpriteConfig, map::TilePosition};

#[derive(Debug, Default, Eq)]
pub struct ItemConfig {
    pub id: u32,
    pub name: Option<String>,
    pub ground_speed: u32,
    // pub minimap_color: Option<u32>,
    pub can_walk: bool,
    pub can_move: bool,
    pub have_fullbank: bool,
    // pub should_avoid: bool,
    pub is_container: bool,
    pub is_cumulative: bool,
    pub top: bool,
}

impl PartialEq for ItemConfig {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub config: Arc<ItemConfig>,
    pub amount: u32,
    pub content: Vec<Arc<Item>>,
    pub capacity: usize,
}

impl Item {
    pub fn new(config: Arc<ItemConfig>, amount: u32, capacity: usize) -> Self {
        Item {
            config,
            amount,
            content: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn get_patterns(&self, pos: &TilePosition, sprite: &SpriteConfig) -> (u32, u32, u32) {
        if self.config.is_cumulative && sprite.pattern_x == 4 && sprite.pattern_y == 2 {
            if self.amount < 5 {
                return (self.amount - 1, 0, 0);
            } else if self.amount < 10 {
                return (0, 1, 0);
            } else if self.amount < 25 {
                return (1, 1, 0);
            } else if self.amount < 50 {
                return (2, 1, 0);
            } else {
                return (3, 1, 0);
            }
        }

        let x = pos.x % sprite.pattern_x;
        let y = pos.y % sprite.pattern_y;
        let z = pos.floor % sprite.pattern_z;
        (x, y, z)
    }
}
