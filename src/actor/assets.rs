use std::collections::HashMap;
use std::fs;

use bevy::prelude::*;
use serde_json::*;

#[derive(Resource, Default)]
pub struct Outfits {
    pub outfits: HashMap<u32, Outfit>,
}

pub struct Outfit {
    pub id: u32,
    pub still_sprite_id: u32,
    pub moving_sprite_id: u32,
    pub sprite_group: String,
}

pub fn read_outfits_config() -> Outfits {
    let Ok(contents) = fs::read_to_string("assets/configs/outfit_conf.json") else {
        panic!("Could not read sprites file");
    };
    let outfits_conf: Value = serde_json::from_str(&contents).unwrap();

    let mut outfits_map = HashMap::new();

    for out in outfits_conf.as_array().unwrap().iter() {
        let id = out["id"].as_u64().unwrap() as u32;
        let still_sprite_id = out["still_sprite_id"].as_u64().unwrap() as u32;
        let moving_sprite_id = out["moving_sprite_id"].as_u64().unwrap() as u32;
        let sprite_group = out["group"].as_str().unwrap().to_string();

        outfits_map.insert(
            id,
            Outfit {
                id,
                still_sprite_id,
                moving_sprite_id,
                sprite_group,
            },
        );
    }

    return Outfits {
        outfits: outfits_map,
    };
}
