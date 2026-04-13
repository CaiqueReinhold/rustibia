use std::sync::Arc;

use bevy::{platform::collections::HashMap, prelude::*};

use crate::{
    actor::AgentId,
    items::{InventorySlot, Item},
};

#[derive(Component)]
pub struct Player {
    pub agent_id: AgentId,
}

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
