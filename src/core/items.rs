use bevy::{ecs::resource::Resource, log::info};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use crate::items::{InventorySlot, ItemConfig, ItemFlag, ItemId};

#[derive(Resource, Debug)]
pub struct ItemConfigs {
    pub items: HashMap<ItemId, Arc<ItemConfig>>,
}

pub fn read_item_configs() -> HashMap<ItemId, Arc<ItemConfig>> {
    let Ok(contents) = fs::read_to_string("assets/configs/items.json") else {
        panic!("Could not read items file");
    };
    let item_configs: Value = serde_json::from_str(&contents).unwrap();
    let mut item_map: HashMap<ItemId, Arc<ItemConfig>> = HashMap::new();
    for conf in item_configs.as_array().unwrap().iter() {
        if let Some(item) = read_item_config(conf) {
            item_map.insert(item.id, item);
        } else {
            panic!("Could not read items file");
        }
    }

    item_map
}

fn read_item_config(config: &Value) -> Option<Arc<ItemConfig>> {
    let id = config["id"].as_u64()? as ItemId;
    // let name = if config["name"].is_null() {
    //     None
    // } else {
    //     Some(config["name"].as_str()?.to_string())
    // };
    // let minimap_color = None;
    let friction = Some(config["ground_speed"].as_u64()? as u8);
    let slot = if config["slot"].is_null() {
        None
    } else {
        Some(match config["slot"].as_u64()? {
            0 => InventorySlot::BothHands,
            1 => InventorySlot::Head,
            2 => InventorySlot::Amulet,
            3 => InventorySlot::Backpack,
            4 => InventorySlot::Chest,
            5 => InventorySlot::RightHand,
            6 => InventorySlot::LeftHand,
            7 => InventorySlot::Legs,
            8 => InventorySlot::Feet,
            9 => InventorySlot::Ring,
            10 => InventorySlot::Trinket,
            _ => {
                info!("{:?}", config);
                return None;
            }
        })
    };
    let mut flags: Vec<ItemFlag> = Vec::new();
    if config["is_ground"].as_bool()? {
        flags.push(ItemFlag::Ground);
    }
    if config["is_border"].as_bool()? {
        flags.push(ItemFlag::Border);
    }
    if !config["can_walk"].as_bool()? {
        flags.push(ItemFlag::Unpass);
    }
    if config["fullbank"].as_bool()? {
        flags.push(ItemFlag::FullBank);
    }
    if config["top"].as_bool()? {
        flags.push(ItemFlag::Top);
    }
    if config["is_container"].as_bool()? {
        flags.push(ItemFlag::Container);
    }
    if !config["can_move"].as_bool()? {
        flags.push(ItemFlag::Unmove);
    }
    if config["cumulative"].as_bool()? {
        flags.push(ItemFlag::Cumulative);
    }
    if config["can_take"].as_bool()? {
        flags.push(ItemFlag::Take);
    }
    if config["bottom"].as_bool()? {
        flags.push(ItemFlag::Bottom);
    }
    if config["usable"].as_bool()? {
        flags.push(ItemFlag::Usable);
    }
    Some(Arc::new(ItemConfig {
        id,
        // name,
        flags,
        friction,
        slot,
        // minimap_color,
    }))
}
