use std::sync::Arc;

use bevy::prelude::*;

use crate::conf::{
    agent::{SPEED_PARAM_A, SPEED_PARAM_B, SPEED_PARAM_C},
    server::TICK_DURATION_MS,
};
use crate::core::SpriteConfig;

pub type AgentId = u16;

#[derive(Debug, Clone, Copy, Default)]
pub enum Mounted {
    #[default]
    Unmounted = 0,
    Mounted = 1,
}

impl From<Mounted> for u32 {
    fn from(value: Mounted) -> Self {
        value as u32
    }
}

#[derive(Component, Debug, Default)]
pub struct Agent {
    // pub outfit_id: u32,
    pub agent_id: AgentId,
    pub direction: FacingDirection,
    pub addons: u8,
    pub mounted: Mounted,
    pub outfit_colors: (u8, u8, u8, u8),
    pub speed: u16,
    pub boxes: [[Rect; 4]; 2],
}

impl Agent {
    pub fn get_step_duration(&self, tile_friction: u8, is_diagonal: bool) -> u32 {
        let move_speed = (SPEED_PARAM_A * ((self.speed as f32) + SPEED_PARAM_B).ln()
            + SPEED_PARAM_C)
            .round()
            .max(1.0);

        let mut tile_speed = (1000.0 * (tile_friction as f32) / move_speed).floor();
        if is_diagonal {
            tile_speed *= 2.5;
        }
        let tile_speed_tick =
            (tile_speed / (TICK_DURATION_MS as f32)).ceil() * (TICK_DURATION_MS as f32);

        tile_speed_tick as u32
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FacingDirection {
    #[default]
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl From<FacingDirection> for u32 {
    fn from(value: FacingDirection) -> Self {
        value as u32
    }
}

impl From<FacingDirection> for usize {
    fn from(value: FacingDirection) -> Self {
        value as usize
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WalkingDirection {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl WalkingDirection {
    pub fn is_diagonal(self) -> bool {
        matches!(
            self,
            WalkingDirection::NorthEast
                | WalkingDirection::NorthWest
                | WalkingDirection::SouthEast
                | WalkingDirection::SouthWest
        )
    }

    pub fn facing(&self) -> FacingDirection {
        match self {
            WalkingDirection::North => FacingDirection::North,
            WalkingDirection::East => FacingDirection::East,
            WalkingDirection::South => FacingDirection::South,
            WalkingDirection::West => FacingDirection::West,
            WalkingDirection::NorthEast => FacingDirection::East,
            WalkingDirection::SouthEast => FacingDirection::East,
            WalkingDirection::NorthWest => FacingDirection::West,
            WalkingDirection::SouthWest => FacingDirection::West,
        }
    }
}

// --- HUD components ---
#[derive(Component, Debug, Clone)]
pub enum HealthState {
    Lowest,
    Low,
    Half,
    AmostFull,
    Full,
}

impl HealthState {
    pub fn color(&self) -> Color {
        match self {
            HealthState::Full => Srgba::rgb(0.0, 0.7372549, 0.0).into(),
            HealthState::AmostFull => Srgba::rgb(0.6039216, 0.8039216, 0.19607843).into(),
            HealthState::Half => Srgba::rgb(0.98039216, 0.92156863, 0.0).into(),
            HealthState::Low => Srgba::rgb(1.0, 0.5, 0.0).into(),
            HealthState::Lowest => Srgba::rgb(1.0, 0.0, 0.0).into(),
        }
    }

    pub fn from_ratio(ratio: f32) -> Self {
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
}

#[derive(Component, Debug, Clone)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

impl Health {
    pub fn ratio(&self) -> f32 {
        self.current as f32 / self.max as f32
    }
}

#[derive(Component, Debug, Clone)]
pub struct Mana {
    pub current: u32,
    pub max: u32,
}

impl Mana {
    pub fn ratio(&self) -> f32 {
        self.current as f32 / self.max as f32
    }
}

#[derive(Component, Debug, Clone)]
pub struct Hud;

#[derive(Component, Debug, Clone)]
pub struct AgentHud {
    pub main_entity: Entity,
    pub health_bar: Option<Entity>,
    pub mana_bar: Option<Entity>,
    pub display_name: Entity,
    pub world_y_offset: f32,
}

#[derive(Component, Debug, Clone)]
pub struct DisplayName;

#[derive(Component, Debug, Clone)]
pub struct HudBar {
    pub ratio: f32,
}
// --- HUD components ---

#[derive(Component)]
pub struct AgentAnimConfigs {
    pub still: Arc<SpriteConfig>,
    pub moving: Arc<SpriteConfig>,
}
