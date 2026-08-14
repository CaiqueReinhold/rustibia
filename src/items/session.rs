use bevy::prelude::*;

use crate::core::InstanceManager;
use crate::items::container::PreventContainerCloseEvent;
use crate::items::instancing::{ChangedTileQueue, ItemState, SpawnedItem};
use crate::items::material::ItemInstance;

/// Despawns the session's ground items and clears everything that indexed them.
///
/// The floor entities they hang off are `Startup`-spawned and stay; only their
/// children go. UI copies of items live inside windows and die with the `MainUI`
/// tree.
pub(super) fn cleanup_session(mut commands: Commands, items: Query<Entity, With<SpawnedItem>>) {
    for entity in &items {
        commands.entity(entity).despawn();
    }
    commands.insert_resource(InstanceManager::<ItemInstance>::default());
    commands.insert_resource(ItemState::default());
    commands.insert_resource(ChangedTileQueue::default());
    commands.remove_resource::<PreventContainerCloseEvent>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Position;
    use bevy::ecs::system::RunSystemOnce;

    fn seeded_world() -> World {
        let mut world = World::new();
        world.init_resource::<InstanceManager<ItemInstance>>();
        world.init_resource::<ItemState>();
        world.init_resource::<ChangedTileQueue>();
        world
    }

    /// Ground items are children of the floor entities, which survive the session.
    /// Nothing else despawns them.
    #[test]
    fn cleanup_despawns_every_spawned_item() {
        let mut world = seeded_world();
        let item = world.spawn(SpawnedItem).id();

        world.run_system_once(cleanup_session).unwrap();

        assert!(world.get_entity(item).is_err());
    }

    /// `occupied_tiles` maps positions to entities that no longer exist after the
    /// despawn above; a stale entry makes the next session's first tile update
    /// address a dead entity.
    #[test]
    fn cleanup_clears_the_tile_bookkeeping() {
        let mut world = seeded_world();
        let item = world.spawn(SpawnedItem).id();
        world
            .resource_mut::<ItemState>()
            .occupied_tiles
            .insert(Position { x: 1, y: 2, z: 7 }, item);
        world
            .resource_mut::<ChangedTileQueue>()
            .changed_positions
            .push_back(Position { x: 1, y: 2, z: 7 });

        world.run_system_once(cleanup_session).unwrap();

        assert!(world.resource::<ItemState>().occupied_tiles.is_empty());
        assert!(
            world
                .resource::<ChangedTileQueue>()
                .changed_positions
                .is_empty()
        );
    }
}
