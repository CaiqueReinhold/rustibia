use std::sync::Arc;

use bevy::{platform::collections::HashMap, prelude::*};

use crate::items::{InventorySlot, Item};

#[derive(Component)]
pub struct Player;

#[derive(Resource, Debug)]
pub struct PlayerInventory {
    pub items: HashMap<InventorySlot, Arc<Item>>,
    pub capacity: u32,
}

impl PlayerInventory {
    pub fn get_capacity_display(&self) -> String {
        format!("{}", self.capacity / 100)
    }
}
