use std::sync::Arc;

use crate::{
    conf::map::{CONTAINER_COORD_FLAG, INVENTORY_COORD_FLAG},
    core::SpriteConfig,
    items::ContainerId,
    map::Position,
};

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
    pub fn as_id(&self) -> u16 {
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

impl ItemPlacement {
    pub fn to_wire_position(&self) -> Position {
        match self {
            ItemPlacement::Map { position, .. } => position.clone(),
            ItemPlacement::Container { container_id, slot } => Position {
                x: CONTAINER_COORD_FLAG,
                y: *container_id,
                z: *slot as u8,
            },
            ItemPlacement::Inventory { slot } => Position {
                x: INVENTORY_COORD_FLAG,
                y: slot.as_id(),
                z: 0,
            },
        }
    }

    pub fn wire_stack_index(&self) -> u8 {
        match self {
            ItemPlacement::Map { index, .. } => *index as u8,
            _ => 0,
        }
    }
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
    MultiUse,
    ForceUse,
    Avoid,
    BlockSight,
}

#[derive(Debug, Eq)]
pub struct ItemConfig {
    pub id: ItemId,
    pub flags: Vec<ItemFlag>,
    pub friction: Option<u8>,
    pub slot: Option<InventorySlot>,
    pub minimap_color: Option<u8>,
    pub elevation: Option<u8>,
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

        let x = pos.x as u32 % sprite.pattern_x;
        let y = pos.y as u32 % sprite.pattern_y;
        let z = pos.z as u32 % sprite.pattern_z;
        (x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conf::map::{CONTAINER_COORD_FLAG, INVENTORY_COORD_FLAG};

    #[test]
    fn map_placement_encodes_as_its_position() {
        let p = ItemPlacement::Map {
            position: Position {
                x: 100,
                y: 200,
                z: 7,
            },
            index: 3,
        };
        assert_eq!(
            p.to_wire_position(),
            Position {
                x: 100,
                y: 200,
                z: 7
            }
        );
        assert_eq!(p.wire_stack_index(), 3);
    }

    #[test]
    fn container_placement_encodes_flag_id_slot() {
        let p = ItemPlacement::Container {
            container_id: 5,
            slot: 9,
        };
        assert_eq!(
            p.to_wire_position(),
            Position {
                x: CONTAINER_COORD_FLAG,
                y: 5,
                z: 9
            }
        );
        assert_eq!(p.wire_stack_index(), 0);
    }

    #[test]
    fn inventory_placement_encodes_flag_slot() {
        let p = ItemPlacement::Inventory {
            slot: InventorySlot::Head,
        };
        assert_eq!(
            p.to_wire_position(),
            Position {
                x: INVENTORY_COORD_FLAG,
                y: InventorySlot::Head.as_id(),
                z: 0
            }
        );
        assert_eq!(p.wire_stack_index(), 0);
    }
}
