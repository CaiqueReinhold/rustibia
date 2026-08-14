use bevy::prelude::*;

use crate::agent::components::{Agent, Hud};
use crate::agent::material::AgentInstance;
use crate::core::InstanceManager;

/// Despawns the session's agents and frees their GPU instance slots.
///
/// HUDs are only collected here while they are still unparented: once
/// `attach_huds_to_viewport` has run they are children of the game viewport and
/// die with the `MainUI` tree, and despawning them twice in the same transition
/// logs an error.
pub(super) fn cleanup_session(
    mut commands: Commands,
    agents: Query<Entity, With<Agent>>,
    orphan_huds: Query<Entity, (With<Hud>, Without<ChildOf>)>,
) {
    for entity in &agents {
        commands.entity(entity).despawn();
    }
    for entity in &orphan_huds {
        commands.entity(entity).despawn();
    }
    commands.insert_resource(InstanceManager::<AgentInstance>::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// Agents are root entities in the world, not children of anything the UI
    /// despawn sweep reaches — if this doesn't despawn them, the previous
    /// session's characters are still standing there in the next one.
    #[test]
    fn cleanup_despawns_every_agent() {
        let mut world = World::new();
        world.init_resource::<InstanceManager<AgentInstance>>();
        let agent = world.spawn(Agent::default()).id();

        world.run_system_once(cleanup_session).unwrap();

        assert!(world.get_entity(agent).is_err());
    }

    /// A HUD that has already been parented to the game viewport dies with the
    /// `MainUI` tree. One spawned this frame has no parent yet, so nothing else
    /// would ever collect it.
    #[test]
    fn cleanup_despawns_unparented_huds_only() {
        let mut world = World::new();
        world.init_resource::<InstanceManager<AgentInstance>>();
        let parent = world.spawn_empty().id();
        let attached = world.spawn((Hud, ChildOf(parent))).id();
        let orphan = world.spawn(Hud).id();

        world.run_system_once(cleanup_session).unwrap();

        assert!(world.get_entity(orphan).is_err(), "orphan HUD collected");
        assert!(
            world.get_entity(attached).is_ok(),
            "an attached HUD is the UI sweep's to despawn — despawning it here \
             would double-despawn it"
        );
    }
}
