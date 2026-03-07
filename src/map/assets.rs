use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use crate::conf::map::STACK_MAX_VISIBLE_ITEMS;
use crate::items::{Item, ItemConfig};
use crate::map::{
    position::TilePosition,
    storage::{Map, MapTile},
};

pub fn read_map_config() -> Map {
    let Ok(contents) = fs::read_to_string("assets/configs/map_conf.json") else {
        panic!("Could not read map file");
    };
    let map_json: serde_json::Value = serde_json::from_str(&contents).unwrap();

    let mut configs: HashMap<u32, Arc<ItemConfig>> = HashMap::new();
    let mut tiles: HashMap<TilePosition, MapTile> = HashMap::new();

    for cfg in map_json["tile_config"].as_array().unwrap().iter() {
        let id = cfg["id"].as_u64().unwrap() as u32;
        let ground_speed = cfg["ground_speed"].as_u64().unwrap() as u32;
        let can_walk = cfg["can_walk"].as_bool().unwrap();
        // let have_fullbank = cfg["fullbank"].as_bool().unwrap();
        // let should_avoid = cfg["avoid"].as_bool().unwrap();
        // let minimap_color = match &cfg["minimap_color"] {
        //     Value::Number(n) => Some(n.as_u64().unwrap() as u32),
        //     _ => None,
        // };
        let top = cfg["top"].as_bool().unwrap();
        let is_container = false;

        configs.insert(
            id,
            Arc::new(ItemConfig {
                id,
                ground_speed,
                can_walk,
                // have_fullbank,
                // should_avoid,
                // minimap_color,
                is_container,
                top,
                ..Default::default()
            }),
        );
    }

    for tile in map_json["tiles"].as_array().unwrap().iter() {
        let x = tile["x"].as_u64().unwrap() as u32;
        let y = tile["y"].as_u64().unwrap() as u32;
        let z = tile["z"].as_u64().unwrap() as u32;

        let ground = tile["ground_id"]
            .as_u64()
            .map(|id| configs.get(&(id as u32)).unwrap().clone());
        let border = tile["border_id"]
            .as_u64()
            .map(|id| configs.get(&(id as u32)).unwrap().clone());

        let mut items = Vec::with_capacity(STACK_MAX_VISIBLE_ITEMS as usize);
        for item in tile["items"].as_array().unwrap().iter() {
            let config = configs
                .get(&(item["id"].as_u64().unwrap() as u32))
                .unwrap()
                .clone();
            let amount = item["amount"].as_u64().unwrap() as u32;
            items.push(Item { config, amount })
        }

        let pos = TilePosition::new(x, y, z);
        tiles.insert(
            pos,
            MapTile {
                ground,
                border,
                items,
            },
        );
    }

    // let width = map_json["width"].as_u64().unwrap() as u32;
    // let height = map_json["height"].as_u64().unwrap() as u32;
    let floors = map_json["floors"].as_u64().unwrap() as u32;

    Map { floors, tiles }
}
