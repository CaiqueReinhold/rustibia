use std::time::Duration;

#[derive(Debug)]
pub enum SpriteAnimation {
    Static,
    Uniform {
        phase_count: u32,
        phase_duration: Duration,
    },
}

impl SpriteAnimation {
    pub fn total_animation_phases(&self) -> u32 {
        match self {
            SpriteAnimation::Static => 1,
            SpriteAnimation::Uniform { phase_count, .. } => *phase_count,
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
    pub bounding_box: u32,
}
