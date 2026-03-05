use std::time::Duration;

use bevy::prelude::*;

use crate::actor::actor::{Actor, FacingDirection};
use crate::conf::actor::{SPEED_PARAM_A, SPEED_PARAM_B, SPEED_PARAM_C};
use crate::conf::server::TICK_DURATION_MS;
use crate::conf::z_order::ACTOR_Z_OFFSET;
use crate::map::TilePosition;

#[derive(Copy, Clone, Debug)]
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

impl Actor {
    pub fn get_tile_speed(&self, tile_modifier: u32, is_diagonal: bool) -> u32 {
        let move_speed = (SPEED_PARAM_A * ((self.speed as f32) + SPEED_PARAM_B).ln()
            + SPEED_PARAM_C)
            .round()
            .max(1.0);

        let mut tile_speed = (1000.0 * (tile_modifier as f32) / move_speed).floor();
        if is_diagonal {
            tile_speed = tile_speed / 2.0;
        }
        let tile_speed_tick =
            (tile_speed / (TICK_DURATION_MS as f32)).ceil() * (TICK_DURATION_MS as f32);

        return tile_speed_tick as u32;
    }
}

#[derive(Component, Debug)]
pub struct QueuedMove {
    pub start: TilePosition,
    pub end: TilePosition,
    pub step_time_ms: u32,
    pub facing: FacingDirection,
}

#[derive(Component, Debug)]
pub struct Moving {
    pub start: TilePosition,
    pub end: TilePosition,
    pub timer: Timer,
    pub queued: Option<QueuedMove>,
}

pub fn move_actor(
    mut commands: Commands,
    mut moving_q: Query<(Entity, &mut Transform, &mut Moving), With<Actor>>,
    mut actor_q: Query<&mut Actor>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut moving) in moving_q.iter_mut() {
        moving.timer.tick(time.delta());
        if moving.timer.is_finished() {
            let mut actor = actor_q.get_mut(entity).unwrap();
            actor.set_changed();

            commands
                .entity(entity)
                .insert(moving.end.clone())
                .remove::<Moving>();

            match &moving.queued {
                Some(q) => {
                    commands.entity(entity).insert(Moving {
                        start: q.start.clone(),
                        end: q.end.clone(),
                        timer: Timer::new(
                            Duration::from_millis(q.step_time_ms as u64),
                            TimerMode::Once,
                        ),
                        queued: None,
                    });
                    actor.direction = q.facing;
                }
                None => (),
            };
        }

        let start = moving.start.to_world();
        let end = moving.end.to_world();
        let mut interpolated = start.lerp(end, moving.timer.fraction());
        interpolated.z = interpolated.z + ACTOR_Z_OFFSET;
        transform.translation = interpolated;
    }
}
