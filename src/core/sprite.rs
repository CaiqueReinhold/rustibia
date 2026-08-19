use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageLoaderSettings;
use bevy::prelude::*;
use serde_json::*;

use crate::items::ItemId;

pub type OutfitId = u16;
pub type OutfitColors = (u8, u8, u8, u8);

#[derive(Debug)]
pub struct OutfitSprite {
    // pub id: OutfitId,
    pub still_sprite: Arc<SpriteConfig>,
    pub moving_sprite: Arc<SpriteConfig>,
}

#[derive(Resource, Debug)]
pub struct Appearances {
    sheets: HashMap<String, SpriteSheet>,
    items: HashMap<ItemId, Arc<SpriteConfig>>,
    outfits: HashMap<OutfitId, OutfitSprite>,
    asset_server: AssetServer,
}

impl Appearances {
    pub(super) fn new(
        sheets: HashMap<String, SpriteSheet>,
        items: HashMap<u16, Arc<SpriteConfig>>,
        outfits: HashMap<u16, OutfitSprite>,
        asset_server: AssetServer,
    ) -> Self {
        Appearances {
            sheets,
            items,
            outfits,
            asset_server,
        }
    }

    /// Every id reaching here came from an [`crate::items::Item`], and an
    /// `Item` is only built after its id was found in `ItemConfigs` — so a
    /// miss means items.json and sprite.json disagree, which is a broken
    /// install rather than anything the server said.
    pub fn get_item(&self, id: ItemId) -> Arc<SpriteConfig> {
        Arc::clone(
            self.items.get(&id).unwrap_or_else(|| {
                panic!("item {id} is in items.json but missing from sprite.json")
            }),
        )
    }

    pub fn get_outfit(&self, id: OutfitId) -> Option<&OutfitSprite> {
        self.outfits.get(&id)
    }

    pub fn get_sheet(&self, group: &str) -> &SpriteSheet {
        let sheet = self.sheets.get(group).unwrap();
        sheet.texture.get_or_init(|| {
            self.asset_server.load_with_settings::<Image, _>(
                format!("sheets/{}", sheet.sheet_name),
                |s: &mut ImageLoaderSettings| {
                    s.asset_usage = RenderAssetUsages::RENDER_WORLD;
                },
            )
        });
        sheet
    }
}

#[derive(Debug)]
pub struct SpriteSheet {
    pub sheet_name: String,
    pub grid_size: Vec2,
    pub sprite_size: Vec2,
    texture: OnceLock<Handle<Image>>,
}

impl SpriteSheet {
    pub fn texture(&self) -> &Handle<Image> {
        self.texture
            .get()
            .expect("sprite sheet texture not initialized — access via Appearances::get_sheet")
    }
}

// #[derive(Debug)]
// pub enum AnimationLoopType {
//     Infinite,
//     PingPong,
// }

#[derive(Debug)]
pub enum SpriteAnimation {
    Static,
    Uniform {
        // loop_type: AnimationLoopType,
        phase_count: u32,
        phase_duration: Duration,
    },
    NonUniform {
        // loop_type: AnimationLoopType,
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
    pub id: u16,
    pub group: String,
    pub pattern_x: u32,
    pub pattern_y: u32,
    pub pattern_z: u32,
    pub layers: u32,
    pub sprite_ids: Vec<u32>,
    pub animation: SpriteAnimation,
    pub boxes: Vec<Rect>,
    pub shift: Vec2,
}

pub fn read_sprites_config() -> (
    HashMap<ItemId, Arc<SpriteConfig>>,
    HashMap<OutfitId, OutfitSprite>,
) {
    let Ok(contents) = fs::read_to_string("assets/configs/sprite.json") else {
        panic!("Could not read sprites file");
    };
    let sprites: Value = serde_json::from_str(&contents).unwrap();

    let mut items: HashMap<ItemId, Arc<SpriteConfig>> = HashMap::new();
    for conf in sprites["items"].as_array().unwrap().iter() {
        let sprite = read_sprite_config(conf);
        items.insert(sprite.id, Arc::new(sprite));
    }
    let mut outfits: HashMap<OutfitId, OutfitSprite> = HashMap::new();
    for out in sprites["outfits"].as_array().unwrap().iter() {
        let id = out["id"].as_u64().unwrap() as OutfitId;
        let still_sprite = read_sprite_config(&out["still_sprite"]);
        let moving_sprite = read_sprite_config(&out["moving_sprite"]);
        outfits.insert(
            id,
            OutfitSprite {
                // id,
                still_sprite: Arc::new(still_sprite),
                moving_sprite: Arc::new(moving_sprite),
            },
        );
    }

    (items, outfits)
}

fn read_sprite_config(conf: &Value) -> SpriteConfig {
    let id = conf["id"].as_u64().unwrap() as u16;
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

    // Absent for the great majority of appearances; `(0, 0)` then means "draw where
    // the bounding box says", which is what the flag's absence means in the protobuf.
    let shift = match &conf["shift"] {
        Value::Array(s) => Vec2::new(s[0].as_u64().unwrap() as f32, s[1].as_u64().unwrap() as f32),
        _ => Vec2::ZERO,
    };

    SpriteConfig {
        id,
        group,
        pattern_x,
        pattern_y,
        pattern_z,
        layers,
        sprite_ids,
        boxes,
        animation,
        shift,
    }
}

fn read_animation(value: &Value) -> SpriteAnimation {
    match value {
        Value::Null => SpriteAnimation::Static,
        Value::Object(anim) => {
            // let loop_type = if anim.get("loop_type").unwrap().as_str().unwrap() == "INFINITE" {
            //     AnimationLoopType::Infinite
            // } else {
            //     AnimationLoopType::PingPong
            // };
            match &anim["phases"] {
                Value::Array(anim_phases) => {
                    let mut phases: Vec<UVec2> = Vec::new();
                    for phase in anim_phases.iter() {
                        phases.push(UVec2::new(
                            phase[0].as_u64().unwrap() as u32,
                            phase[1].as_u64().unwrap() as u32,
                        ));
                    }
                    SpriteAnimation::NonUniform { phases }
                }
                _ => {
                    let phase_count = anim["phase_count"].as_u64().unwrap() as u32;
                    let phase_duration =
                        Duration::from_millis(anim["phase_duration"].as_u64().unwrap());
                    SpriteAnimation::Uniform {
                        phase_count,
                        phase_duration,
                    }
                }
            }
        }
        _ => SpriteAnimation::Static,
    }
}

pub fn read_sprite_sheets() -> HashMap<String, SpriteSheet> {
    let Ok(contents) = fs::read_to_string("assets/configs/sheets.json") else {
        panic!("Could not read sheets file");
    };
    let sheets: Value = serde_json::from_str(&contents).unwrap();

    let mut sheets_map: HashMap<String, SpriteSheet> = HashMap::new();

    for sheet in sheets.as_array().unwrap().iter() {
        let grid_size = Vec2::new(
            sheet["grid_size"][0].as_u64().unwrap() as f32,
            sheet["grid_size"][1].as_u64().unwrap() as f32,
        );
        let sprite_size = Vec2::new(
            sheet["sprite_size"][0].as_u64().unwrap() as f32,
            sheet["sprite_size"][1].as_u64().unwrap() as f32,
        );
        let sheet_name = sheet["sheet_name"].as_str().unwrap().to_string();
        let group = sheet["group"].as_str().unwrap().to_string();

        sheets_map.insert(
            group,
            SpriteSheet {
                sheet_name,
                grid_size,
                sprite_size,
                texture: OnceLock::new(),
            },
        );
    }

    sheets_map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_json(extra: &str) -> Value {
        serde_json::from_str(&format!(
            r#"{{
                "id": 1, "group": "g",
                "pattern_x": 4, "pattern_y": 1, "pattern_z": 1, "layers": 1,
                "sprite_ids": [0, 1, 2, 3],
                "animation": null,
                "boxes": [[0, 0, 32, 32], [0, 0, 32, 32], [0, 0, 32, 32], [0, 0, 32, 32]]
                {extra}
            }}"#
        ))
        .unwrap()
    }

    /// `shift` is the appearance's art displacement (OTClient's `m_displacement`).
    /// Player outfits carry `(8, 8)`; it is what the agent shader subtracts.
    #[test]
    fn shift_is_read_from_the_config() {
        let config = read_sprite_config(&config_json(r#", "shift": [8, 8]"#));

        assert_eq!(config.shift, Vec2::new(8.0, 8.0));
    }

    /// The flag is absent for most appearances — the demon among them, which is why
    /// a hard-coded padding fitted the player and pushed the demon off its tile.
    /// Absent must mean zero, not "keep the last value" and not a panic.
    #[test]
    fn a_missing_shift_is_zero() {
        let config = read_sprite_config(&config_json(""));

        assert_eq!(config.shift, Vec2::ZERO);
    }

    /// A non-zero shift must not disturb the bounding box it is applied on top of:
    /// the two are independent inputs to `calculate_world_pos`.
    #[test]
    fn shift_does_not_disturb_the_boxes() {
        let shifted = read_sprite_config(&config_json(r#", "shift": [8, 8]"#));
        let unshifted = read_sprite_config(&config_json(""));

        assert_eq!(shifted.boxes, unshifted.boxes);
    }
}
