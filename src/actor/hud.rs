use bevy::prelude::*;

pub enum HealthState {
    Lowest,
    Low,
    Half,
    AmostFull,
    Full,
}

#[derive(Component)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

impl Health {
    pub fn state(&self) -> HealthState {
        let ratio = self.ratio();

        if ratio >= 0.90 {
            HealthState::Full
        } else if ratio >= 0.6 {
            HealthState::AmostFull
        } else if ratio >= 0.3 {
            HealthState::Half
        } else if ratio >= 0.5 {
            HealthState::Low
        } else {
            HealthState::Lowest
        }
    }

    fn ratio(&self) -> f32 {
        self.current as f32 / self.max as f32
    }
}

#[derive(Component)]
pub struct Mana {
    pub current: u32,
    pub max: u32,
}
