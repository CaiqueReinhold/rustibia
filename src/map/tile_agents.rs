use bevy::prelude::*;

use crate::agent::Agent;
use crate::map::{Map, Position};

/// Where an agent is currently recorded in `Map`'s tile index. Needed because a
/// `Changed<Position>` only tells us the new tile, and we must remove the agent
/// from the old one.
#[derive(Component, Debug)]
pub struct IndexedOn(pub Position);

/// Keeps `MapTile::agents` in step with agents' `Position` components.
///
/// Derived from `Changed<Position>` rather than hooked into the spawn / move /
/// teleport events, because `Position` is written from more places than those
/// three and a missed site would silently corrupt the index. Nothing holds
/// `&mut Position` for bookkeeping, so this `Changed` filter really does gate.
pub fn sync_tile_agents(
    mut commands: Commands,
    mut map: ResMut<Map>,
    moved: Query<(Entity, &Agent, &Position, Option<&IndexedOn>), Changed<Position>>,
) {
    for (entity, agent, pos, indexed) in moved.iter() {
        if let Some(IndexedOn(old)) = indexed {
            if old == pos {
                continue;
            }
            map.unindex_agent(agent.agent_id, old);
        }
        map.index_agent(agent.agent_id, pos);
        commands.entity(entity).insert(IndexedOn(pos.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn at(x: u16, y: u16) -> Position {
        Position { x, y, z: 7 }
    }

    fn world_with_map() -> World {
        let mut world = World::new();
        world.insert_resource(Map::default());
        world
    }

    #[test]
    fn an_agent_is_indexed_onto_its_tile() {
        let mut world = world_with_map();
        world.spawn((
            Agent {
                agent_id: 5,
                ..Default::default()
            },
            at(10, 10),
        ));

        world.run_system_once(sync_tile_agents).unwrap();

        assert_eq!(world.resource::<Map>().agents_on(&at(10, 10)), &[5]);
    }

    #[test]
    fn moving_an_agent_reindexes_it() {
        let mut world = world_with_map();
        let e = world
            .spawn((
                Agent {
                    agent_id: 5,
                    ..Default::default()
                },
                at(10, 10),
            ))
            .id();
        world.run_system_once(sync_tile_agents).unwrap();

        world.entity_mut(e).insert(at(11, 10));
        world.run_system_once(sync_tile_agents).unwrap();

        assert!(world.resource::<Map>().agents_on(&at(10, 10)).is_empty());
        assert_eq!(world.resource::<Map>().agents_on(&at(11, 10)), &[5]);
    }

    /// Topmost is the last entry, mirroring `Map::peek_item`. Push order is
    /// arrival order, so the most recently arrived agent is on top.
    #[test]
    fn co_located_agents_keep_arrival_order() {
        let mut world = world_with_map();
        world.spawn((
            Agent {
                agent_id: 1,
                ..Default::default()
            },
            at(10, 10),
        ));
        world.run_system_once(sync_tile_agents).unwrap();

        world.spawn((
            Agent {
                agent_id: 2,
                ..Default::default()
            },
            at(10, 10),
        ));
        world.run_system_once(sync_tile_agents).unwrap();

        assert_eq!(world.resource::<Map>().agents_on(&at(10, 10)), &[1, 2]);
        assert_eq!(world.resource::<Map>().topmost_agent(&at(10, 10)), Some(2));
    }

    /// A tile nobody stands on answers empty rather than panicking — the gesture
    /// asks about arbitrary hovered tiles.
    #[test]
    fn an_empty_tile_has_no_agents() {
        let world = world_with_map();
        assert!(world.resource::<Map>().agents_on(&at(1, 1)).is_empty());
        assert_eq!(world.resource::<Map>().topmost_agent(&at(1, 1)), None);
    }
}
