use bevy::prelude::*;

use crate::agent::AgentId;
use crate::conf::map::TILE_SIZE;
use crate::conf::target::{SQUARE_COLOR, SQUARE_THICKNESS};
use crate::conf::z_order::TARGET_SQUARE_LOCAL_Z;
use crate::map::Map;
use crate::network::events::TargetChanged;

/// The agent this player is attacking, as a session-local `AgentId`.
///
/// Applied **optimistically**: the click writes it before the server answers, and
/// `TargetChanged` overwrites it unconditionally afterwards. This is a correctness
/// requirement, not a responsiveness preference — because the click toggles, an
/// ack-gated resource lets a lagging player clear the target by clicking twice.
/// Divergence is nearly unreachable (the only rejections are an agent the client
/// already removed, and self-targeting, which the gesture never sends), so no
/// rollback machinery is needed.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub struct CombatTarget(pub Option<AgentId>);

impl CombatTarget {
    /// Applies a locally-predicted value.
    pub fn set_locally(&mut self, agent_id: Option<AgentId>) {
        self.0 = agent_id;
    }

    /// What clicking `agent_id` should produce: clicking the current target
    /// clears it, clicking anything else selects it.
    pub fn next_for_click(&self, agent_id: AgentId) -> Option<AgentId> {
        if self.0 == Some(agent_id) {
            None
        } else {
            Some(agent_id)
        }
    }

    /// Applies a click optimistically and returns what to tell the server.
    ///
    /// Deciding and applying in one call is deliberate: the caller cannot obtain
    /// a value to send without having already applied it, so "optimistic before
    /// send" is a property of this function rather than of statement order in the
    /// gesture handler.
    pub fn apply_click(&mut self, agent_id: AgentId) -> Option<AgentId> {
        let next = self.next_for_click(agent_id);
        self.set_locally(next);
        next
    }
}

/// The server's answer always wins.
pub fn on_target_changed(
    event: On<TargetChanged>,
    mut commands: Commands,
    mut target: ResMut<CombatTarget>,
    map: Res<Map>,
    square_q: Query<Entity, With<TargetSquare>>,
) {
    target.0 = event.agent_id;
    refresh_target_square(&mut commands, &target, &map, &square_q);
}

#[derive(Component)]
pub struct TargetSquare;

/// `to_world()` is the tile's top-left corner; the square must sit on the tile
/// centre. Y is negative because world Y grows upward while tile Y grows down.
pub fn square_centre_offset() -> Vec2 {
    Vec2::new(TILE_SIZE / 2.0, -TILE_SIZE / 2.0)
}

/// The four edges of the outline as `(size, centre_offset_from_tile_centre)`.
fn square_bars() -> [(Vec2, Vec2); 4] {
    let half = TILE_SIZE / 2.0;
    let inset = half - SQUARE_THICKNESS / 2.0;
    [
        (
            Vec2::new(TILE_SIZE, SQUARE_THICKNESS),
            Vec2::new(0.0, inset),
        ),
        (
            Vec2::new(TILE_SIZE, SQUARE_THICKNESS),
            Vec2::new(0.0, -inset),
        ),
        (
            Vec2::new(SQUARE_THICKNESS, TILE_SIZE),
            Vec2::new(-inset, 0.0),
        ),
        (
            Vec2::new(SQUARE_THICKNESS, TILE_SIZE),
            Vec2::new(inset, 0.0),
        ),
    ]
}

/// Despawns any existing square and, if there is a target, spawns a new one as a
/// **child of the target agent's entity** — which gives walk-offset tracking for
/// free, with no per-frame sync system.
pub fn refresh_target_square(
    commands: &mut Commands,
    target: &CombatTarget,
    map: &Map,
    square_q: &Query<Entity, With<TargetSquare>>,
) {
    for existing in square_q.iter() {
        commands.entity(existing).despawn();
    }

    let Some(agent_id) = target.0 else {
        return;
    };
    let Some(agent_entity) = map.get_agent(agent_id) else {
        return;
    };

    let centre = square_centre_offset();
    commands.entity(agent_entity).with_children(|parent| {
        let mut root = parent.spawn((
            TargetSquare,
            Transform::from_xyz(centre.x, centre.y, TARGET_SQUARE_LOCAL_Z),
            Visibility::default(),
        ));
        root.with_children(|frame| {
            for (size, offset) in square_bars() {
                frame.spawn((
                    Sprite {
                        color: SQUARE_COLOR,
                        custom_size: Some(size),
                        ..Default::default()
                    },
                    Transform::from_xyz(offset.x, offset.y, 0.0),
                ));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_target_starts_empty() {
        assert_eq!(CombatTarget::default().0, None);
    }

    #[test]
    fn set_locally_applies_immediately() {
        let mut target = CombatTarget::default();
        target.set_locally(Some(7));
        assert_eq!(target.0, Some(7));
    }

    /// The toggle is what makes optimistic application necessary: with ack-gating,
    /// a lagging player clicks twice and the second click clears the target they
    /// were trying to set.
    #[test]
    fn next_for_click_toggles_the_current_target_off() {
        let mut target = CombatTarget::default();
        target.set_locally(Some(7));
        assert_eq!(target.next_for_click(7), None);
    }

    #[test]
    fn next_for_click_switches_to_a_different_agent() {
        let mut target = CombatTarget::default();
        target.set_locally(Some(7));
        assert_eq!(target.next_for_click(9), Some(9));
    }

    #[test]
    fn next_for_click_sets_when_nothing_is_targeted() {
        let target = CombatTarget::default();
        assert_eq!(target.next_for_click(7), Some(7));
    }

    /// The optimistic apply is the point: `apply_click` cannot hand you something
    /// to send without having already applied it, so the ordering cannot be got
    /// wrong at the call site.
    #[test]
    fn apply_click_applies_before_returning_what_to_send() {
        let mut target = CombatTarget::default();
        let to_send = target.apply_click(7);
        assert_eq!(to_send, Some(7));
        assert_eq!(target.0, Some(7), "applied before the caller can send");

        let to_send = target.apply_click(7);
        assert_eq!(to_send, None);
        assert_eq!(target.0, None);
    }

    fn world_with_observer() -> World {
        let mut world = World::new();
        world.init_resource::<CombatTarget>();
        world.insert_resource(Map::default());
        world.add_observer(on_target_changed);
        world
    }

    /// The server is authoritative: its reply overwrites whatever the click guessed.
    #[test]
    fn a_server_reply_overwrites_an_optimistic_value() {
        let mut world = world_with_observer();
        world.resource_mut::<CombatTarget>().set_locally(Some(7));

        world.trigger(TargetChanged { agent_id: Some(9) });
        world.flush();

        assert_eq!(world.resource::<CombatTarget>().0, Some(9));
    }

    /// Including when the server's answer is a rejection or a viewport-exit clear.
    #[test]
    fn a_server_clear_overwrites_an_optimistic_value() {
        let mut world = world_with_observer();
        world.resource_mut::<CombatTarget>().set_locally(Some(7));

        world.trigger(TargetChanged { agent_id: None });
        world.flush();

        assert_eq!(world.resource::<CombatTarget>().0, None);
    }

    #[test]
    fn the_square_is_a_full_tile_outline() {
        // Four bars, each spanning a full tile edge, at the configured thickness.
        let bars = square_bars();
        assert_eq!(bars.len(), 4);
        for (size, _) in &bars {
            assert!(
                (size.x - TILE_SIZE).abs() < f32::EPSILON
                    || (size.y - TILE_SIZE).abs() < f32::EPSILON,
                "every bar spans one full tile edge, got {size:?}"
            );
            assert!(
                (size.x - SQUARE_THICKNESS).abs() < f32::EPSILON
                    || (size.y - SQUARE_THICKNESS).abs() < f32::EPSILON,
                "every bar is one thickness deep, got {size:?}"
            );
        }
    }

    /// `to_world()` returns the tile's TOP-LEFT corner and the convention is
    /// size-dependent: 64px sprites centre on it, 32px things do not. This is the
    /// counterpart of OTClient's `- getDisplacement()`; both put the square on the
    /// tile rather than on the artwork. Getting this wrong shipped a visible bug
    /// last session.
    #[test]
    fn the_square_is_offset_to_the_tile_centre() {
        assert_eq!(
            square_centre_offset(),
            Vec2::new(TILE_SIZE / 2.0, -TILE_SIZE / 2.0)
        );
    }

    /// The square is a child of the agent, and transform hierarchies compose
    /// additively. Using the absolute offset as the local z would put the square
    /// at world_z + AGENT_Z_OFFSET + TARGET_SQUARE_Z_OFFSET — in FRONT of the
    /// creature and above TOP_Z_OFFSET — which is the opposite of what OTClient
    /// does and what the constant's name promises.
    ///
    /// This drives the real call site (`on_target_changed` ->
    /// `refresh_target_square`) rather than just re-deriving the constants: it
    /// spawns a stand-in agent entity with the transform real agents carry
    /// (`AGENT_Z_OFFSET` baked into local z, per `agent/movement.rs`), triggers
    /// `TargetChanged`, and reads back the *actual* local z the square was
    /// spawned with. A version of this test that only checked
    /// `AGENT_Z_OFFSET + TARGET_SQUARE_LOCAL_Z == TARGET_SQUARE_Z_OFFSET` would
    /// be a tautology — true by construction of the constants, regardless of
    /// which constant the call site actually uses — so it would not have caught
    /// the original bug. Confirmed this version does: swapping
    /// `TARGET_SQUARE_LOCAL_Z` for `TARGET_SQUARE_Z_OFFSET` at the call site
    /// makes it fail (composed 0.024, not under `AGENT_Z_OFFSET`).
    #[test]
    fn the_square_composes_to_just_under_the_agent() {
        use crate::conf::z_order::{AGENT_Z_OFFSET, TARGET_SQUARE_Z_OFFSET, TOP_Z_OFFSET};

        let mut world = World::new();
        world.init_resource::<CombatTarget>();
        let mut map = Map::default();
        let agent_entity = world
            .spawn(Transform::from_xyz(0.0, 0.0, AGENT_Z_OFFSET))
            .id();
        map.add_agent(7, agent_entity);
        world.insert_resource(map);
        world.add_observer(on_target_changed);

        world.trigger(TargetChanged { agent_id: Some(7) });
        world.flush();

        let square_entity = world
            .query_filtered::<Entity, With<TargetSquare>>()
            .single(&world)
            .expect("a target square was spawned");
        let square_local_z = world.get::<Transform>(square_entity).unwrap().translation.z;
        let composed = AGENT_Z_OFFSET + square_local_z;

        assert!(
            (composed - TARGET_SQUARE_Z_OFFSET).abs() < f32::EPSILON,
            "composed {composed} should equal {TARGET_SQUARE_Z_OFFSET}, got local z {square_local_z}"
        );
        assert!(
            composed < AGENT_Z_OFFSET,
            "the square draws under the creature"
        );
        assert!(composed < TOP_Z_OFFSET, "and under top-layer items");
    }

    /// `refresh_target_square` despawns any existing square before spawning a new
    /// one. Without that despawn, switching targets leaves the old square behind
    /// as an orphaned child of the previous target — this drives the real
    /// observer across two targets in a row and pins that only the newest
    /// square survives, parented under the newest target.
    #[test]
    fn switching_targets_leaves_exactly_one_square() {
        use crate::conf::z_order::AGENT_Z_OFFSET;

        let mut world = World::new();
        world.init_resource::<CombatTarget>();
        let mut map = Map::default();
        let first_agent = world
            .spawn(Transform::from_xyz(0.0, 0.0, AGENT_Z_OFFSET))
            .id();
        let second_agent = world
            .spawn(Transform::from_xyz(32.0, 0.0, AGENT_Z_OFFSET))
            .id();
        map.add_agent(7, first_agent);
        map.add_agent(9, second_agent);
        world.insert_resource(map);
        world.add_observer(on_target_changed);

        world.trigger(TargetChanged { agent_id: Some(7) });
        world.flush();
        world.trigger(TargetChanged { agent_id: Some(9) });
        world.flush();

        let squares: Vec<Entity> = world
            .query_filtered::<Entity, With<TargetSquare>>()
            .iter(&world)
            .collect();
        assert_eq!(squares.len(), 1, "exactly one square should exist");

        let parent = world.get::<ChildOf>(squares[0]).unwrap().parent();
        assert_eq!(
            parent, second_agent,
            "the surviving square should be parented under the newest target"
        );
    }
}
