use bevy::prelude::*;

use crate::player::components::PlayerInventory;
use crate::player::interaction::{
    ContainerNavTarget, InteractionMode, MouseHoverState, PendingWalkAction,
};
use crate::player::movement::{MovementQueue, PlayerElevation};
use crate::player::pathfinding::AutoWalkTarget;

/// Clears everything about the character that just left.
///
/// The player entity itself is an `Agent` and is despawned by the agent cleanup.
/// `Keybinds` is user configuration, not session state, and stays. `KeyRepeatState`
/// is re-seeded by re-running `keyboard::init_repeat_state`, which is registered
/// into the same set.
pub(super) fn cleanup_session(mut commands: Commands) {
    commands.insert_resource(MovementQueue::default());
    commands.insert_resource(PlayerElevation::default());
    commands.insert_resource(MouseHoverState::default());
    commands.insert_resource(InteractionMode::default());
    commands.remove_resource::<PlayerInventory>();
    commands.remove_resource::<AutoWalkTarget>();
    commands.remove_resource::<PendingWalkAction>();
    commands.remove_resource::<ContainerNavTarget>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_ui::WindowId;
    use crate::items::{InventorySlot, ItemPlacement};
    use crate::map::Position;
    use bevy::ecs::system::RunSystemOnce;

    fn seeded_world() -> World {
        let mut world = World::new();
        world.init_resource::<MovementQueue>();
        world.init_resource::<PlayerElevation>();
        world.init_resource::<MouseHoverState>();
        world.init_resource::<InteractionMode>();
        world
    }

    /// Every one of these describes a character that is no longer in the world.
    /// `PlayerInventory` in particular is inserted by `spawn_player`, so its
    /// absence is what tells the rest of the client there is no player yet.
    #[test]
    fn cleanup_drops_the_per_character_resources() {
        let mut world = seeded_world();
        world.insert_resource(PlayerInventory {
            items: default(),
            capacity: 400,
        });
        world.insert_resource(AutoWalkTarget(Position { x: 1, y: 1, z: 7 }));
        world.insert_resource(ContainerNavTarget(WindowId::new()));

        world.run_system_once(cleanup_session).unwrap();

        assert!(world.get_resource::<PlayerInventory>().is_none());
        assert!(world.get_resource::<AutoWalkTarget>().is_none());
        assert!(world.get_resource::<ContainerNavTarget>().is_none());
    }

    /// A half-finished drag or an in-flight targeting cursor must not be waiting
    /// for the player when they log back in.
    #[test]
    fn cleanup_returns_the_interaction_mode_to_idle() {
        let mut world = seeded_world();
        *world.resource_mut::<InteractionMode>() = InteractionMode::Targeting {
            source: ItemPlacement::Inventory {
                slot: InventorySlot::Head,
            },
            source_item_id: 1,
        };

        world.run_system_once(cleanup_session).unwrap();

        assert!(matches!(
            *world.resource::<InteractionMode>(),
            InteractionMode::Idle
        ));
    }
}
