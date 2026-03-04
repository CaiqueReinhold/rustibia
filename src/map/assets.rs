use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use crate::conf::map::STACK_MAX_VISIBLE_ITEMS;
use crate::items::{Item, ItemConfig};
use crate::map::{
    map::{Map, MapTile},
    position::TilePosition,
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
        let sprite_id = cfg["sprite_id"].as_u64().unwrap() as u32;
        let ground_speed = cfg["ground_speed"].as_u64().unwrap() as u32;
        let can_walk = cfg["can_walk"].as_bool().unwrap();
        let have_fullbank = cfg["fullbank"].as_bool().unwrap();
        let should_avoid = cfg["avoid"].as_bool().unwrap();
        let minimap_color = cfg["minimap_color"].as_u64().unwrap() as u32;
        let is_container = false;

        configs.insert(
            id,
            Arc::new(ItemConfig {
                id,
                sprite_id,
                ground_speed,
                can_walk,
                have_fullbank,
                should_avoid,
                minimap_color,
                is_container,
                ..Default::default()
            }),
        );
    }

    for tile in map_json["tiles"].as_array().unwrap().iter() {
        let x = tile["x"].as_u64().unwrap() as u32;
        let y = tile["y"].as_u64().unwrap() as u32;
        let z = tile["z"].as_u64().unwrap() as u32;

        let ground = match tile["ground_id"].as_u64() {
            Some(id) => Some(configs.get(&(id as u32)).unwrap().clone()),
            None => None,
        };
        let border = match tile["border_id"].as_u64() {
            Some(id) => Some(configs.get(&(id as u32)).unwrap().clone()),
            None => None,
        };

        let mut items = Vec::with_capacity(STACK_MAX_VISIBLE_ITEMS as usize);
        for item_id in tile["items"].as_array().unwrap().iter() {
            let config = configs
                .get(&(item_id.as_u64().unwrap() as u32))
                .unwrap()
                .clone();
            items.push(Item { config, amount: 1 })
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

    let width = map_json["width"].as_u64().unwrap() as u32;
    let height = map_json["height"].as_u64().unwrap() as u32;
    let floors = map_json["floors"].as_u64().unwrap() as u32;

    return Map {
        width,
        height,
        floors,
        tiles,
    };
}
