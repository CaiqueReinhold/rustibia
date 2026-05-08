use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;

use crate::agent::components::Agent;
use crate::agent::{AgentId, WalkingDirection};
use crate::conf::z_order::AGENT_Z_OFFSET;
use crate::map::{Map, Position};
use crate::network::events::{AgentChangedDirection, TeleportAgent};

#[derive(Component, Debug)]
pub struct Moving {
    pub start: Position,
    pub end: Position,
    pub timer: Timer,
}

#[derive(Component, Debug)]
pub struct ShouldTeleport {
    pub position: Position,
}

#[derive(Component, Debug, Default)]
pub struct MoveQueue(pub VecDeque<(Position, WalkingDirection)>);

#[derive(Event, Debug)]
pub struct StartAgentMove {
    pub agent_id: AgentId,
    pub direction: WalkingDirection,
}

#[derive(Event, Debug)]
pub struct UpdateElevation {
    pub pos: Position,
}

pub fn on_start_agent_move(
    event: On<StartAgentMove>,
    mut commands: Commands,
    mut agent_q: Query<(&mut Agent, &Position)>,
    map: Res<Map>,
) {
    let Some(entity) = map.get_agent(event.agent_id) else {
        return;
    };
    let Ok((mut agent, position)) = agent_q.get_mut(entity) else {
        return;
    };

    let start_position = position.clone();
    let facing = event.direction.facing();
    let end_position = start_position.clone() + event.direction;
    let tile_modifier = map.get_tile_friction(&end_position).unwrap_or(0);
    let step_time_ms = agent.get_step_duration(tile_modifier, event.direction.is_diagonal());

    info!(
        "agent {} -> move from {} to {}. Tile friction: {}. step_time_ms: {}",
        event.agent_id, start_position, end_position, tile_modifier, step_time_ms
    );

    agent.direction = facing;
    commands.entity(entity).insert(Moving {
        start: start_position.clone(),
        end: end_position,
        timer: Timer::new(Duration::from_millis(step_time_ms as u64), TimerMode::Once),
    });
}

pub fn on_agent_change_direction(
    event: On<AgentChangedDirection>,
    map: Res<Map>,
    mut agent_q: Query<&mut Agent>,
) {
    if let Some(agent_entity) = map.get_agent(event.agent_id)
        && let Ok(mut agent) = agent_q.get_mut(agent_entity)
        && agent.direction != event.facing
    {
        agent.direction = event.facing;
    }
}

pub fn move_agent(
    mut commands: Commands,
    mut moving_q: Query<(Entity, &mut Transform, &mut Moving), With<Agent>>,
    mut agent_q: Query<&mut Agent>,
    map: Res<Map>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut moving) in moving_q.iter_mut() {
        moving.timer.tick(time.delta());
        if moving.timer.is_finished() {
            let mut agent = agent_q.get_mut(entity).unwrap();
            agent.set_changed();

            commands
                .entity(entity)
                .insert(moving.end.clone())
                .remove::<Moving>();

            let elevation = map.get_elevation(&moving.end);
            transform.translation =
                moving.end.to_world_with_elevation(elevation) + vec3(0.0, 0.0, AGENT_Z_OFFSET);
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
        let end = moving.end.to_world();
        let interpolated = start.lerp(end, moving.timer.fraction()) + elevation;
        transform.translation = Vec3::new(
            interpolated.x.round(),
            interpolated.y.round(),
            f32::lerp(start.z, end.z, moving.timer.fraction()) + AGENT_Z_OFFSET,
        );
    }
}

pub fn on_update_elevation(
    event: On<UpdateElevation>,
    mut moving_q: Query<(&mut Transform, &Position), With<Agent>>,
    map: Res<Map>,
) {
    let elevation = map.get_elevation(&event.pos);
    for (mut transform, position) in moving_q.iter_mut() {
        if *position == event.pos {
            transform.translation =
                position.to_world_with_elevation(elevation) + vec3(0.0, 0.0, AGENT_Z_OFFSET);
        }
    }
}

pub fn on_teleport_agent(event: On<TeleportAgent>, mut commands: Commands, map: Res<Map>) {
    if let Some(agent) = map.get_agent(event.agent_id) {
        commands.entity(agent).insert(ShouldTeleport {
            position: event.position.clone(),
        });
    }
}

pub fn teleport_agents(
    mut commands: Commands,
    mut agents_q: Query<(Entity, &ShouldTeleport, &mut Transform, Option<&Moving>)>,
    map: Res<Map>,
) {
    for (entity, teleport, mut transform, moving) in agents_q.iter_mut() {
        if moving.is_none() {
            let elevation = map.get_elevation(&teleport.position);
            transform.translation = teleport.position.to_world_with_elevation(elevation);
            commands
                .entity(entity)
                .insert(teleport.position.clone())
                .remove::<ShouldTeleport>();
        }
    }
}

pub fn process_agent_move_queues(
    mut commands: Commands,
    mut queue_q: Query<(Entity, &Agent, &Position, &mut MoveQueue), Without<Moving>>,
) {
    for (entity, agent, position, mut queue) in &mut queue_q {
        let Some((move_from, direction)) = queue.0.pop_front() else {
            continue;
        };

        if *position != move_from {
            if queue.0.is_empty() {
                commands.entity(entity).insert(move_from);
            } else {
                return;
            }
        }

        commands.trigger(StartAgentMove {
            agent_id: agent.agent_id,
            direction,
        });
    }
}
