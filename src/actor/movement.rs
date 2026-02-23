use bevy::prelude::*;

use crate::actor::actor::{Actor, FacingDirection};
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

#[derive(Component, Debug)]
pub struct Moving {
    pub start: TilePosition,
    pub end: TilePosition,
    pub timer: Timer,
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
            commands
                .entity(entity)
                .insert(moving.end.clone())
                .remove::<Moving>();

            let mut actor = actor_q.get_mut(entity).unwrap();
            actor.set_changed();

            continue;
        }

        let start = moving.start.to_world();
        let end = moving.end.to_world();
        let mut interpolated = start.lerp(end, moving.timer.fraction());
        interpolated.z = transform.translation.z;
        transform.translation = interpolated;
    }
}
