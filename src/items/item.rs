use std::sync::Arc;

use crate::{core::SpriteConfig, map::Position};

pub type ItemId = u16;

#[derive(Debug, PartialEq, Eq)]
pub enum ItemFlag {
    Ground,
    Border,
    Container,
    Cumulative,
    Top,
    Unpass,
    Unmove,
    Take,
    FullBank,
    Bottom,
    Usable,
}

#[derive(Debug, Eq)]
pub struct ItemConfig {
    pub id: ItemId,
    // pub name: Option<String>,
    pub flags: Vec<ItemFlag>,
    pub friction: Option<u8>,
    // pub minimap_color: Option<u8>,
}

impl ItemConfig {
    pub fn has_flag(&self, flag: ItemFlag) -> bool {
        self.flags.contains(&flag)
    }
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
}

impl Item {
    pub fn new(config: Arc<ItemConfig>, amount: u32) -> Self {
        Item { config, amount }
    }

    pub fn get_patterns(&self, pos: &Position, sprite: &SpriteConfig) -> (u32, u32, u32) {
        if self.config.has_flag(ItemFlag::Cumulative)
            && sprite.pattern_x == 4
            && sprite.pattern_y == 2
        {
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
        let z = pos.z % sprite.pattern_z;
        (x, y, z)
    }
}
