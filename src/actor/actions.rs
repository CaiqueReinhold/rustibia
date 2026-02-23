use std::time::Duration;

use bevy::prelude::*;

use crate::{
    actor::{
        actor::Actor,
        movement::{Moving, WalkingDirection},
        player::Player,
    },
    map::{Map, TilePosition},
};

#[derive(Event, Debug)]
pub struct PlayerMove {
    pub direction: WalkingDirection,
}

pub fn on_player_move(
    event: On<PlayerMove>,
    mut commands: Commands,
    mut player_q: Query<(Entity, &mut Actor, Option<&Moving>, &TilePosition), With<Player>>,
    map: Res<Map>,
) {
    let Ok((entity, mut actor, moving, position)) = player_q.single_mut() else {
        return;
    };

    if moving.is_some() {
        return;
    }

    actor.direction = event.direction.facing();
    let end_postion = match event.direction {
        WalkingDirection::North => position.delta(0, -1),
        WalkingDirection::East => position.delta(1, 0),
        WalkingDirection::South => position.delta(0, 1),
        WalkingDirection::West => position.delta(-1, 0),
        WalkingDirection::NorthEast => position.delta(1, -1),
        WalkingDirection::SouthEast => position.delta(1, 1),
        WalkingDirection::NorthWest => position.delta(-1, -1),
        WalkingDirection::SouthWest => position.delta(-1, -1),
    };

    if map.can_move(&end_postion) {
        let step_time_ms =
            map.get_step_duration_ms(&end_postion, actor.speed, event.direction.is_diagonal());
        commands.entity(entity).insert(Moving {
            start: position.clone(),
            end: end_postion,
            timer: Timer::new(Duration::from_millis(step_time_ms as u64), TimerMode::Once),
        });
    }
}
