use std::collections::HashMap;

use bevy::prelude::*;

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
