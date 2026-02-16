use bevy::prelude::*;

use std::collections::HashMap;
use std::fs;

#[derive(Debug)]
pub struct GroundConfig {
    pattern_x: u8,
    pattern_y: u8,
    sprite_ids: Vec<u32>,
    ground_speed: u32,
    can_walk: bool,
    animation_frames: u8,
    animation_duration: u32,
    is_opaque: bool,
    fullbank: bool,
    avoid: bool,
    minimap_color: u32,
}

#[derive(Debug)]
pub struct Tile {
    pub sprite_id: u32,
    pub frame_count: u32,
    pub sheet_id: usize,
}

#[derive(Resource)]
pub struct WorldMap {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub ground: HashMap<(u32, u32, u32), Tile>,
    pub borders: HashMap<(u32, u32, u32), Tile>,
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
        appearances.insert(
            app["id"].as_i64().unwrap() as u32,
            GroundConfig {
                pattern_x: app["pattern_x"].as_i64().unwrap() as u8,
                pattern_y: app["pattern_y"].as_i64().unwrap() as u8,
                sprite_ids: app["sprite_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap() as u32)
                    .collect(),
                ground_speed: app["ground_speed"].as_i64().unwrap() as u32,
                can_walk: app["can_walk"].as_bool().unwrap(),
                animation_frames: app["animation_frames"].as_i64().unwrap() as u8,
                animation_duration: app["animation_duration"].as_i64().unwrap() as u32,
                is_opaque: app["is_opaque"].as_bool().unwrap(),
                fullbank: app["fullbank"].as_bool().unwrap(),
                avoid: app["avoid"].as_bool().unwrap(),
                minimap_color: app["minimap_color"].as_i64().unwrap() as u32,
            },
        );
    }

    let mut ground: HashMap<(u32, u32, u32), Tile> = HashMap::new();
    let mut borders: HashMap<(u32, u32, u32), Tile> = HashMap::new();

    fn get_sprite_index(x: u32, y: u32, sprite_config: &GroundConfig) -> usize {
        let pat_x = x % (sprite_config.pattern_x as u32);
        let pat_y = y % (sprite_config.pattern_y as u32);

        info!("{},{}: {:?}", x, y, sprite_config);

        (pat_y * (sprite_config.pattern_x as u32) + pat_x) as usize
    }

    for tile in map_json["ground"].as_array().unwrap().iter() {
        let x = tile["x"].as_u64().unwrap() as u32;
        let y = tile["y"].as_u64().unwrap() as u32;
        let z = tile["z"].as_u64().unwrap() as u32;
        let tile_id = tile["id"].as_u64().unwrap() as u32;

        let sprite_config = appearances.get(&tile_id).unwrap();

        ground.insert(
            (x, y, z),
            Tile {
                sprite_id: sprite_config.sprite_ids[get_sprite_index(x, y, sprite_config)],
                frame_count: sprite_config.animation_frames as u32,
                sheet_id: 1,
            },
        );
    }

    for tile in map_json["borders"].as_array().unwrap().iter() {
        let x = tile["x"].as_u64().unwrap() as u32;
        let y = tile["y"].as_u64().unwrap() as u32;
        let z = tile["z"].as_u64().unwrap() as u32;
        let tile_id = tile["id"].as_u64().unwrap() as u32;

        let sprite_config = appearances.get(&tile_id).unwrap();

        borders.insert(
            (x, y, z),
            Tile {
                sprite_id: sprite_config.sprite_ids[get_sprite_index(x, y, sprite_config)],
                frame_count: sprite_config.animation_frames as u32,
                sheet_id: 1,
            },
        );
    }

    commands.insert_resource(WorldMap {
        width: map_json["width"].as_i64().unwrap() as u32,
        height: map_json["width"].as_i64().unwrap() as u32,
        depth: 15,
        ground,
        borders,
    });
}
