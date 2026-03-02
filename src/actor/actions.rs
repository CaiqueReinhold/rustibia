use std::time::Duration;

use bevy::prelude::*;

use crate::{
    actor::{
        actor::{Actor, FacingDirection},
        movement::{Moving, QueuedMove, WalkingDirection},
        player::Player,
    },
    map::{Map, TilePosition},
};

#[derive(Event, Debug)]
pub struct PlayerMove {
    pub direction: WalkingDirection,
}

#[derive(Event, Debug)]
pub struct PlayerChangeDirection {
    pub direction: FacingDirection,
}

pub fn on_player_move(
    event: On<PlayerMove>,
    mut commands: Commands,
    mut player_q: Query<(Entity, &mut Actor, Option<&mut Moving>, &TilePosition), With<Player>>,
    map: Res<Map>,
) {
    let Ok((entity, mut actor, moving, position)) = player_q.single_mut() else {
        return;
    };

    let start_position = match &moving {
        Some(m) => m.end.clone(),
        None => position.clone(),
    };
    let facing = event.direction.facing();
    let end_postion = match event.direction {
        WalkingDirection::North => start_position.delta(0, -1),
        WalkingDirection::East => start_position.delta(1, 0),
        WalkingDirection::South => start_position.delta(0, 1),
        WalkingDirection::West => start_position.delta(-1, 0),
        WalkingDirection::NorthEast => start_position.delta(1, -1),
        WalkingDirection::SouthEast => start_position.delta(1, 1),
        WalkingDirection::NorthWest => start_position.delta(-1, -1),
        WalkingDirection::SouthWest => start_position.delta(-1, -1),
    };

    if map.can_move(&end_postion) {
        let step_time_ms =
            map.get_step_duration_ms(&end_postion, actor.speed, event.direction.is_diagonal());
        match moving {
            Some(mut m) => {
                m.queued = Some(QueuedMove {
                    start: start_position.clone(),
                    end: end_postion,
                    step_time_ms,
                    facing,
                })
            }
            None => {
                actor.direction = facing;
                commands.entity(entity).insert(Moving {
                    start: start_position.clone(),
                    end: end_postion,
                    timer: Timer::new(Duration::from_millis(step_time_ms as u64), TimerMode::Once),
                    queued: None,
                });
            }
        }
    }
}

pub fn on_player_change_direction(
    event: On<PlayerChangeDirection>,
    mut player: Single<&mut Actor, With<Player>>,
) {
    if player.direction != event.direction {
        player.direction = event.direction;
    }
}
