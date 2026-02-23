use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use crate::map::{
    map::{Map, MapTile, TileConfig},
    position::TilePosition,
};

pub fn read_map_config() -> Map {
    let Ok(contents) = fs::read_to_string("assets/configs/map_conf.json") else {
        panic!("Could not read map file");
    };
    let map_json: serde_json::Value = serde_json::from_str(&contents).unwrap();

    let mut configs: HashMap<u32, Arc<TileConfig>> = HashMap::new();
    let mut tiles: HashMap<TilePosition, MapTile> = HashMap::new();

    for cfg in map_json["tile_config"].as_array().unwrap().iter() {
        let id = cfg["id"].as_u64().unwrap() as u32;
        let sprite_id = cfg["sprite_id"].as_u64().unwrap() as u32;
        let ground_speed = cfg["ground_speed"].as_u64().unwrap() as u32;
        let can_walk = cfg["can_walk"].as_bool().unwrap();
        let fullbank = cfg["fullbank"].as_bool().unwrap();
        let avoid = cfg["avoid"].as_bool().unwrap();
        let minimap_color = cfg["minimap_color"].as_u64().unwrap() as u32;

        configs.insert(
            id,
            Arc::new(TileConfig {
                id,
                sprite_id,
                ground_speed,
                can_walk,
                fullbank,
                avoid,
                minimap_color,
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

        let pos = TilePosition::new(x, y, z);
        tiles.insert(pos, MapTile { ground, border });
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
