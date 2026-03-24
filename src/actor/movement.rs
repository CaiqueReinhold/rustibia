use std::time::Duration;

use bevy::prelude::*;

use crate::actor::components::{Actor, FacingDirection};
use crate::actor::WalkingDirection;
use crate::conf::z_order::ACTOR_Z_OFFSET;
use crate::map::{Map, TilePosition};
use crate::player::components::Player;

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

#[derive(Event, Debug)]
pub struct MoveActor {
    pub direction: WalkingDirection,
}

#[derive(Event, Debug)]
pub struct ActorChangeDirection {
    pub direction: FacingDirection,
}

pub fn on_actor_move(
    event: On<MoveActor>,
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
    let end_postion = start_position.clone() + event.direction;
    let tile_modifier = map.get_tile_friction(&end_postion);
    let step_time_ms = actor.get_step_duration(tile_modifier, event.direction.is_diagonal());
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

pub fn on_actor_change_direction(
    event: On<ActorChangeDirection>,
    mut player: Single<&mut Actor, With<Player>>,
) {
    if player.direction != event.direction {
        player.direction = event.direction;
    }
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

            transform.translation = moving.end.to_world() + Vec3::new(0.0, 0.0, ACTOR_Z_OFFSET);
            if let Some(q) = &moving.queued {
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
            };
            return;
        }

        let start = moving.start.to_world();
        let end = moving.end.to_world();
        let interpolated = start.lerp(end, moving.timer.fraction());
        transform.translation = Vec3::new(
            interpolated.x.round(),
            interpolated.y.round(),
            f32::max(end.z, start.z) + ACTOR_Z_OFFSET,
        );
    }
}
