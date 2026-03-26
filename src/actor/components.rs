use bevy::prelude::*;

use crate::conf::{
    actor::{SPEED_PARAM_A, SPEED_PARAM_B, SPEED_PARAM_C},
    server::TICK_DURATION_MS,
};

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
pub struct Actor {
    // pub outfit_id: u32,
    pub direction: FacingDirection,
    pub addons: u32,
    pub mounted: Mounted,
    pub color_head: u32,
    pub color_body: u32,
    pub color_legs: u32,
    pub color_feet: u32,
    pub speed: u16,
    pub box_size: [f32; 2],
    pub boxes: [[Rect; 4]; 2],
    pub phase_counts: [u32; 2],
}

impl Actor {
    pub fn get_step_duration(&self, tile_friction: u8, is_diagonal: bool) -> u32 {
        let move_speed = (SPEED_PARAM_A * ((self.speed as f32) + SPEED_PARAM_B).ln()
            + SPEED_PARAM_C)
            .round()
            .max(1.0);

        let mut tile_speed = (1000.0 * (tile_friction as f32) / move_speed).floor();
        if is_diagonal {
            tile_speed /= 2.0;
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
