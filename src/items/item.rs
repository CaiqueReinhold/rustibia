use std::sync::Arc;

use crate::{core::SpriteConfig, items::ContainerId, map::Position};

pub type ItemId = u16;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy, PartialOrd, Ord)]
pub enum InventorySlot {
    Head,
    Amulet,
    Chest,
    Backpack,
    LeftHand,
    RightHand,
    BothHands,
    Ring,
    Legs,
    Feet,
    Trinket,
}

impl InventorySlot {
    pub fn as_id(&self) -> u32 {
        match self {
            InventorySlot::BothHands => 0,
            InventorySlot::Head => 1,
            InventorySlot::Amulet => 2,
            InventorySlot::Backpack => 3,
            InventorySlot::Chest => 4,
            InventorySlot::RightHand => 5,
            InventorySlot::LeftHand => 6,
            InventorySlot::Legs => 7,
            InventorySlot::Feet => 8,
            InventorySlot::Ring => 9,
            InventorySlot::Trinket => 10,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(InventorySlot::BothHands),
            1 => Some(InventorySlot::Head),
            2 => Some(InventorySlot::Amulet),
            3 => Some(InventorySlot::Backpack),
            4 => Some(InventorySlot::Chest),
            5 => Some(InventorySlot::RightHand),
            6 => Some(InventorySlot::LeftHand),
            7 => Some(InventorySlot::Legs),
            8 => Some(InventorySlot::Feet),
            9 => Some(InventorySlot::Ring),
            10 => Some(InventorySlot::Trinket),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ItemPlacement {
    Map {
        position: Position,
        index: usize,
    },
    Container {
        container_id: ContainerId,
        slot: usize,
    },
    Inventory {
        slot: InventorySlot,
    },
}

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
    pub flags: Vec<ItemFlag>,
    pub friction: Option<u8>,
    pub slot: Option<InventorySlot>,
    pub minimap_color: Option<u8>,
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
