use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub max_experience: u32,
    pub experience: u32,
    pub speed: f32,
}
