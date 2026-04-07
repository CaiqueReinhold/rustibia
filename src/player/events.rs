use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

use crate::actor::{spawn_actor, ActorInstance, ActorMaterial, LoadedMaterials};
use crate::conf::actor::{ADDON_1_FLAG, ADDON_2_FLAG};
use crate::core::{Appearances, GameState, InstanceManager, ItemConfigs};

use crate::items::{InventorySlot, Item};
use crate::network::events::{IventorySlotUpdated, SpawnPlayer};
use crate::player::components::{Player, PlayerInventory};

pub fn check_game_ready(mut commands: Commands, player_q: Query<&Player>) {
    if !player_q.is_empty() {
        commands.set_state(GameState::InGame);
    }
}

pub fn spawn_player(
    event: On<SpawnPlayer>,
    mut commands: Commands,
    mut loaded_materials: ResMut<LoadedMaterials>,
    mut materials: ResMut<Assets<ActorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut instances: ResMut<InstanceManager<ActorInstance>>,
    appearances: Res<Appearances>,
    item_configs: Res<ItemConfigs>,
    time: Res<Time>,
) {
    let entity = spawn_actor(
        &mut commands,
        &mut loaded_materials,
        &mut materials,
        &mut meshes,
        &mut buffers,
        &mut instances,
        &appearances,
        &time,
        event.outfit.0,
        event.outfit.1,
        event.speed,
        ADDON_1_FLAG | ADDON_2_FLAG,
        event.position.clone(),
    );

    commands
        .entity(entity)
        .insert((Player, event.health.clone(), event.mana.clone()));

    let mut inventory = HashMap::new();
    if let Some(item_id) = event.inventory_head {
        inventory.insert(
            InventorySlot::Head,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_amulet {
        inventory.insert(
            InventorySlot::Amulet,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_backpack {
        inventory.insert(
            InventorySlot::Backpack,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_chest {
        inventory.insert(
            InventorySlot::Chest,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_right_hand {
        inventory.insert(
            InventorySlot::RightHand,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_left_hand {
        inventory.insert(
            InventorySlot::LeftHand,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_legs {
        inventory.insert(
            InventorySlot::Legs,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_feet {
        inventory.insert(
            InventorySlot::Feet,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_ring {
        inventory.insert(
            InventorySlot::Ring,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }
    if let Some(item_id) = event.inventory_trinket {
        inventory.insert(
            InventorySlot::Trinket,
            Arc::new(Item {
                config: item_configs.items.get(&item_id).unwrap().clone(),
                amount: 1,
            }),
        );
    }

    commands.insert_resource(PlayerInventory { items: inventory });
}

pub fn on_slot_update(
    event: On<IventorySlotUpdated>,
    mut inventory: ResMut<PlayerInventory>,
    item_configs: Res<ItemConfigs>,
) {
    if let Some(item_id) = event.item_id {
        let item = Arc::new(Item {
            config: item_configs.items.get(&item_id).unwrap().clone(),
            amount: 1,
        });
        inventory.items.insert(event.slot, item);
    } else {
        inventory.items.remove(&event.slot);
    }
}
