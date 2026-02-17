use bevy::prelude::*;

use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

#[derive(Debug)]
pub struct GroundConfig {
    pub id: u32,
    pub pattern_x: u32,
    pub pattern_y: u32,
    pub sprite_ids: Vec<u32>,
    pub ground_speed: u32,
    pub can_walk: bool,
    pub animation_frames: u32,
    pub animation_duration: u32,
    pub is_opaque: bool,
    pub fullbank: bool,
    pub avoid: bool,
    pub minimap_color: u32,
}

#[derive(Resource)]
pub struct WorldMap {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub ground: HashMap<(u32, u32, u32), Arc<GroundConfig>>,
    pub borders: HashMap<(u32, u32, u32), Arc<GroundConfig>>,
    pub appearances: Vec<Arc<GroundConfig>>,
}

pub fn load_map_resource(mut commands: Commands) {
    let Ok(contents) = fs::read_to_string("assets/maps/map_ground.json") else {
        panic!("Could not read map file");
    };
    let map_json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    let Ok(contents) = fs::read_to_string("assets/maps/ground_appearances.json") else {
        panic!("Could not read map file");
    };
    let apps_json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    let mut appearances = HashMap::new();

    for app in apps_json.as_array().unwrap().iter() {
        let id = app["id"].as_u64().unwrap() as u32;
        appearances.insert(
            id,
            Arc::new(GroundConfig {
                id,
                pattern_x: app["pattern_x"].as_u64().unwrap() as u32,
                pattern_y: app["pattern_y"].as_u64().unwrap() as u32,
                sprite_ids: app["sprite_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect(),
                ground_speed: app["ground_speed"].as_u64().unwrap() as u32,
                can_walk: app["can_walk"].as_bool().unwrap(),
                animation_frames: app["animation_frames"].as_u64().unwrap() as u32,
                animation_duration: app["animation_duration"].as_u64().unwrap() as u32,
                is_opaque: app["is_opaque"].as_bool().unwrap(),
                fullbank: app["fullbank"].as_bool().unwrap(),
                avoid: app["avoid"].as_bool().unwrap(),
                minimap_color: app["minimap_color"].as_u64().unwrap() as u32,
            }),
        );
    }

    let mut ground: HashMap<(u32, u32, u32), Arc<GroundConfig>> = HashMap::new();
    let mut borders: HashMap<(u32, u32, u32), Arc<GroundConfig>> = HashMap::new();

    for tile in map_json["ground"].as_array().unwrap().iter() {
        let x = tile["x"].as_u64().unwrap() as u32;
        let y = tile["y"].as_u64().unwrap() as u32;
        let z = tile["z"].as_u64().unwrap() as u32;
        let tile_id = tile["id"].as_u64().unwrap() as u32;

        let sprite_config = appearances.get(&tile_id).unwrap();

        ground.insert((x, y, z), sprite_config.clone());
    }

    for tile in map_json["borders"].as_array().unwrap().iter() {
        let x = tile["x"].as_u64().unwrap() as u32;
        let y = tile["y"].as_u64().unwrap() as u32;
        let z = tile["z"].as_u64().unwrap() as u32;
        let tile_id = tile["id"].as_u64().unwrap() as u32;

        let sprite_config = appearances.get(&tile_id).unwrap();

        borders.insert((x, y, z), sprite_config.clone());
    }

    commands.insert_resource(WorldMap {
        width: map_json["width"].as_u64().unwrap() as u32,
        height: map_json["width"].as_u64().unwrap() as u32,
        depth: 15,
        ground,
        borders,
        appearances: appearances
            .values()
            .into_iter()
            .map(|a| a.clone())
            .collect(),
    });
}
