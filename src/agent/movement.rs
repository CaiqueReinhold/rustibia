use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;

use crate::agent::components::Agent;
use crate::agent::{AgentId, WalkingDirection};
use crate::conf::z_order::AGENT_Z_OFFSET;
use crate::map::{FloorEntities, Map, Position};
use crate::network::events::{AgentChangedDirection, TeleportAgent};
use crate::player::components::Player;

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
    mut agent_q: Query<(&mut Agent, &Position, &mut Transform)>,
    map: Res<Map>,
) {
    let Some(entity) = map.get_agent(event.agent_id) else {
        return;
    };
    let Ok((mut agent, position, mut transform)) = agent_q.get_mut(entity) else {
        return;
    };

    let start_position = position.clone();
    let facing = event.direction.facing();
    let end_position = start_position.clone() + event.direction;
    agent.direction = facing;

    let Some(tile_modifier) = map.get_tile_friction(&end_position) else {
        // This client has no friction for the destination, so any step duration
        // would be invented — and a made-up short one used to free the send gate
        // a frame later, straight into a full server cooldown. Place the agent
        // instead, the same way `move_agent` does when a step completes.
        let elevation = map.get_elevation(&end_position);
        transform.translation =
            end_position.to_world_with_elevation(elevation) + vec3(0.0, 0.0, AGENT_Z_OFFSET);
        commands.entity(entity).insert(end_position);
        return;
    };

    let slide_ms = agent.get_step_duration(tile_modifier, false);
    commands.entity(entity).insert(Moving {
        start: start_position,
        end: end_position,
        timer: Timer::new(Duration::from_millis(slide_ms as u64), TimerMode::Once),
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
            continue;
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
    mut agents_q: Query<(
        Entity,
        &ShouldTeleport,
        &mut Transform,
        Option<&Moving>,
        Option<&Player>,
    )>,
    map: Res<Map>,
    floor_ents: Res<FloorEntities>,
) {
    for (entity, teleport, mut transform, moving, player) in agents_q.iter_mut() {
        if moving.is_none() {
            let elevation = map.get_elevation(&teleport.position);
            transform.translation = teleport.position.to_world_with_elevation(elevation);
            commands
                .entity(entity)
                .insert(teleport.position.clone())
                .remove::<ShouldTeleport>();

            if player.is_none() {
                commands.entity(entity).detach_all_related::<ChildOf>();
                commands
                    .entity(floor_ents.floors[teleport.position.z as usize])
                    .add_child(entity);
            }
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
                continue;
            }
        }

        commands.trigger(StartAgentMove {
            agent_id: agent.agent_id,
            direction,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::FacingDirection;
    use crate::items::{Item, ItemConfig, ItemFlag};
    use std::sync::Arc;

    fn at(x: u16, y: u16) -> Position {
        Position { x, y, z: 7 }
    }

    /// An agent can be told to walk onto a tile this client has never been sent —
    /// it happens at the viewport edge. There is no honest step duration for that,
    /// so the agent is placed rather than glided at a made-up speed.
    #[test]
    fn an_unknown_destination_friction_snaps_instead_of_gliding() {
        let mut world = World::new();
        let mut map = Map::default();
        let entity = world
            .spawn((
                Agent {
                    agent_id: 1,
                    speed: 120,
                    ..Default::default()
                },
                at(100, 100),
                Transform::default(),
            ))
            .id();
        map.add_agent(1, entity);
        // No tile is inserted anywhere, so the destination has no friction.
        world.insert_resource(map);
        world.add_observer(on_start_agent_move);

        world.trigger(StartAgentMove {
            agent_id: 1,
            direction: WalkingDirection::East,
        });
        world.flush();

        assert!(
            world.get::<Moving>(entity).is_none(),
            "no interpolation without a real step duration"
        );
        assert_eq!(
            world.get::<Position>(entity),
            Some(&at(101, 100)),
            "the agent still arrives"
        );
        assert_eq!(
            world.get::<Agent>(entity).unwrap().direction,
            FacingDirection::East,
            "and still turns"
        );
    }

    /// OTClient's `updateWalk` drives the slide from `getStepDuration(true)` and only
    /// terminates the walk on the full duration, so a diagonal is drawn at the same speed
    /// as a cardinal and the extra time is spent standing still on the new tile.
    #[test]
    fn a_diagonal_is_drawn_at_cardinal_speed() {
        let ground = Arc::new(ItemConfig {
            id: 1,
            flags: vec![ItemFlag::Ground],
            friction: Some(150),
            slot: None,
            minimap_color: None,
            elevation: None,
        });
        let mut world = World::new();
        let mut map = Map::default();
        for pos in [at(100, 100), at(101, 101), at(101, 100)] {
            map.replace_tile(vec![Arc::new(Item::new(ground.clone(), 1))], &pos);
        }
        let entity = world
            .spawn((
                Agent {
                    agent_id: 1,
                    speed: 120,
                    ..Default::default()
                },
                at(100, 100),
                Transform::default(),
            ))
            .id();
        map.add_agent(1, entity);
        world.insert_resource(map);
        world.add_observer(on_start_agent_move);

        world.trigger(StartAgentMove {
            agent_id: 1,
            direction: WalkingDirection::SouthEast,
        });
        world.flush();

        let diagonal = world.get::<Moving>(entity).unwrap().timer.duration();

        world.trigger(StartAgentMove {
            agent_id: 1,
            direction: WalkingDirection::East,
        });
        world.flush();

        let cardinal = world.get::<Moving>(entity).unwrap().timer.duration();

        assert_eq!(diagonal, Duration::from_millis(500));
        assert_eq!(
            diagonal, cardinal,
            "a diagonal is drawn in the same time as a cardinal, not 2.5x it"
        );
    }
}
