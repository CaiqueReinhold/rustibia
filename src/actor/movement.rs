use std::time::Duration;

use bevy::prelude::*;

use crate::actor::components::Actor;
use crate::actor::WalkingDirection;
use crate::conf::z_order::ACTOR_Z_OFFSET;
use crate::map::{Map, Position};
use crate::network::events::AgentChangedDirection;

#[derive(Component, Debug)]
pub struct Moving {
    pub start: Position,
    pub end: Position,
    pub timer: Timer,
}

#[derive(Event, Debug)]
pub struct MoveActor {
    pub direction: WalkingDirection,
}

pub fn on_actor_move(
    event: On<MoveActor>,
    mut commands: Commands,
    mut player_q: Query<(Entity, &mut Actor, &Position)>,
    map: Res<Map>,
) {
    let Ok((entity, mut actor, position)) = player_q.single_mut() else {
        return;
    };

    let start_position = position.clone();
    let facing = event.direction.facing();
    let end_position = start_position.clone() + event.direction;
    let tile_modifier = map.get_tile_friction(&end_position).unwrap_or(0);
    let step_time_ms = actor.get_step_duration(tile_modifier, event.direction.is_diagonal());

    actor.direction = facing;
    commands.entity(entity).insert(Moving {
        start: start_position.clone(),
        end: end_position,
        timer: Timer::new(Duration::from_millis(step_time_ms as u64), TimerMode::Once),
    });
}

pub fn on_actor_change_direction(
    event: On<AgentChangedDirection>,
    map: Res<Map>,
    mut agent_q: Query<&mut Actor>,
) {
    if let Some(agent_entity) = map.get_agent(event.agent_id) {
        if let Ok(mut agent) = agent_q.get_mut(agent_entity) {
            if agent.direction != event.facing {
                agent.direction = event.facing;
            }
        }
    }
}

pub fn move_actor(
    mut commands: Commands,
    mut moving_q: Query<(Entity, &mut Transform, &mut Moving), With<Actor>>,
    mut actor_q: Query<&mut Actor>,
    map: Res<Map>,
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

            let elevation = map.get_elevation(&moving.end);
            transform.translation =
                moving.end.to_world_with_elevation(elevation) + vec3(0.0, 0.0, ACTOR_Z_OFFSET);
            return;
        }

        let start = moving.start.to_world();
        let elevation = if moving.timer.fraction() > 0.5 {
            let end_elevation = map.get_elevation(&moving.end);
            vec3(-(end_elevation as f32), end_elevation as f32, 0.0)
        } else {
            let start_elevation = map.get_elevation(&moving.start);
            vec3(-(start_elevation as f32), start_elevation as f32, 0.0)
        };
        info!("applyed elevation: {}", elevation);
        let end = moving.end.to_world();
        let interpolated = start.lerp(end, moving.timer.fraction()) + elevation;
        transform.translation = Vec3::new(
            interpolated.x.round(),
            interpolated.y.round(),
            f32::lerp(start.z, end.z, moving.timer.fraction()) + ACTOR_Z_OFFSET,
        );
    }
}
