use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::core::SpriteConfig;

#[derive(Debug)]
pub struct SpriteSheet {
    pub texture: Handle<Image>,
    pub grid_size: Vec2,
}

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
