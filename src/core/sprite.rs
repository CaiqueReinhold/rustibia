use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::*;
use serde_json::*;

#[derive(Resource, Debug)]
pub struct Appearances {
    pub sheets: HashMap<String, SpriteSheet>,
    pub sprite_configs: HashMap<u32, Arc<SpriteConfig>>,
}

impl Appearances {
    pub fn get_group(&self, group: &String) -> impl Iterator<Item = Arc<SpriteConfig>> {
        self.sprite_configs
            .values()
            .filter(|i| &i.group == group)
            .map(|i| i.clone())
            .collect::<Vec<Arc<SpriteConfig>>>()
            .into_iter()
    }
}

#[derive(Debug)]
pub struct SpriteSheet {
    pub texture: Handle<Image>,
    pub grid_size: Vec2,
}

#[derive(Debug)]
pub enum AnimationLoopType {
    Infinite,
    PingPong,
}

#[derive(Debug)]
pub enum SpriteAnimation {
    Static,
    Uniform {
        loop_type: AnimationLoopType,
        phase_count: u32,
        phase_duration: Duration,
    },
    NonUniform {
        loop_type: AnimationLoopType,
        phases: Vec<UVec2>,
    },
}

impl SpriteAnimation {
    pub fn total_animation_phases(&self) -> u32 {
        match self {
            SpriteAnimation::Static => 1,
            SpriteAnimation::Uniform { phase_count, .. } => *phase_count,
            SpriteAnimation::NonUniform { phases, .. } => phases.len() as u32,
        }
    }
}

#[derive(Debug)]
pub struct SpriteConfig {
    pub id: u32,
    pub group: String,
    pub pattern_x: u32,
    pub pattern_y: u32,
    pub pattern_z: u32,
    pub layers: u32,
    pub sprite_ids: Vec<u32>,
    pub animation: SpriteAnimation,
    pub box_size: Vec2,
    pub boxes: Vec<Rect>,
}

pub fn read_sprites_config() -> HashMap<u32, Arc<SpriteConfig>> {
    let Ok(contents) = fs::read_to_string("assets/configs/sprite_conf.json") else {
        panic!("Could not read sprites file");
    };
    let sprites: Value = serde_json::from_str(&contents).unwrap();

    let mut config_map: HashMap<u32, Arc<SpriteConfig>> = HashMap::new();

    for conf in sprites.as_array().unwrap().iter() {
        let id = conf["id"].as_u64().unwrap() as u32;
        let group = conf["group"].as_str().unwrap().to_string();
        let pattern_x = conf["pattern_x"].as_u64().unwrap() as u32;
        let pattern_y = conf["pattern_y"].as_u64().unwrap() as u32;
        let pattern_z = conf["pattern_z"].as_u64().unwrap() as u32;
        let layers = conf["layers"].as_u64().unwrap() as u32;
        let sprite_ids: Vec<u32> = conf["sprite_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_u64().unwrap() as u32)
            .collect();
        let animation = read_animation(&conf["animation"]);
        let box_size = conf["box_size"].as_u64().unwrap() as f32;
        let box_size = Vec2::new(box_size, box_size);
        let mut boxes: Vec<Rect> = Vec::new();
        for b in conf["boxes"].as_array().unwrap().iter() {
            boxes.push(Rect {
                min: Vec2 {
                    x: b[0].as_u64().unwrap() as f32,
                    y: b[1].as_u64().unwrap() as f32,
                },
                max: Vec2 {
                    x: b[2].as_u64().unwrap() as f32,
                    y: b[3].as_u64().unwrap() as f32,
                },
            });
        }

        let spr_conf = Arc::new(SpriteConfig {
            id,
            group,
            pattern_x,
            pattern_y,
            pattern_z,
            layers,
            sprite_ids,
            box_size,
            boxes,
            animation,
        });
        config_map.insert(id, spr_conf);
    }

    return config_map;
}

fn read_animation(value: &Value) -> SpriteAnimation {
    match value {
        Value::Null => SpriteAnimation::Static,
        Value::Object(anim) => {
            let loop_type = if anim.get("loop_type").unwrap().as_str().unwrap() == "INFINITE" {
                AnimationLoopType::Infinite
            } else {
                AnimationLoopType::PingPong
            };
            let animation: SpriteAnimation;
            match &anim["phases"] {
                Value::Array(anim_phases) => {
                    let mut phases: Vec<UVec2> = Vec::new();
                    for phase in anim_phases.iter() {
                        phases.push(UVec2::new(
                            phase[0].as_u64().unwrap() as u32,
                            phase[1].as_u64().unwrap() as u32,
                        ));
                    }
                    animation = SpriteAnimation::NonUniform { loop_type, phases };
                }
                _ => {
                    let phase_count = anim["phase_count"].as_u64().unwrap() as u32;
                    let phase_duration =
                        Duration::from_millis(anim["phase_duration"].as_u64().unwrap());
                    animation = SpriteAnimation::Uniform {
                        loop_type,
                        phase_count,
                        phase_duration,
                    };
                }
            };
            animation
        }
        _ => SpriteAnimation::Static,
    }
}

pub fn read_sprite_sheets(a_server: &AssetServer) -> HashMap<String, SpriteSheet> {
    let Ok(contents) = fs::read_to_string("assets/configs/sprite_sheets.json") else {
        panic!("Could not read sheets file");
    };
    let sheets: Value = serde_json::from_str(&contents).unwrap();

    let mut sheets_map: HashMap<String, SpriteSheet> = HashMap::new();

    for sheet in sheets.as_array().unwrap().iter() {
        let grid_size = Vec2::new(
            sheet["atlas_grid"][0].as_u64().unwrap() as f32,
            sheet["atlas_grid"][1].as_u64().unwrap() as f32,
        );
        let sheet_name = sheet["sheet_name"].as_str().unwrap().to_string();
        let group = sheet["group"].as_str().unwrap().to_string();

        let handle = a_server.load(format!("sprites/{}", sheet_name));

        sheets_map.insert(
            group,
            SpriteSheet {
                texture: handle,
                grid_size,
            },
        );
    }

    return sheets_map;
}
