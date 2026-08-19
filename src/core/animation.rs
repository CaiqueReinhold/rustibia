use bevy::prelude::*;
use std::sync::Arc;
use std::time::Duration;

use crate::core::sprite::{SpriteAnimation, SpriteConfig};

pub const MAX_LAYERS: usize = 6;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnimationSet;

#[derive(Component)]
pub struct SpriteAnimator {
    pub config: Arc<SpriteConfig>,
    pub current_sprite_ids: [u32; MAX_LAYERS],
    pub timer: Timer,
    pub current_phase: u32,
    pub pattern_x: u32,
    pub pattern_y: u32,
    pub pattern_z: u32,
    pub moving_animation: bool,
}

impl SpriteAnimator {
    pub fn new(config: Arc<SpriteConfig>, pattern_x: u32, pattern_y: u32, pattern_z: u32) -> Self {
        let duration = match &config.animation {
            SpriteAnimation::Static => Duration::ZERO,
            SpriteAnimation::Uniform { phase_duration, .. } => *phase_duration,
            SpriteAnimation::NonUniform { .. } => Duration::ZERO,
        };
        let timer = if duration.is_zero() {
            Timer::new(Duration::ZERO, TimerMode::Once)
        } else {
            Timer::new(duration, TimerMode::Repeating)
        };
        let mut s = SpriteAnimator {
            config,
            current_sprite_ids: [0; MAX_LAYERS],
            timer,
            current_phase: 0,
            pattern_x,
            pattern_y,
            pattern_z,
            moving_animation: false,
        };
        resolve_simple_sprite_ids(&mut s);
        s
    }
}

pub fn resolve_simple_sprite_ids(animator: &mut SpriteAnimator) {
    let config = &animator.config;
    let phase = animator.current_phase;
    for layer in 0..config.layers.min(MAX_LAYERS as u32) as usize {
        let index = (((phase * config.pattern_z + animator.pattern_z) * config.pattern_y
            + animator.pattern_y)
            * config.pattern_x
            + animator.pattern_x)
            * config.layers
            + layer as u32;
        animator.current_sprite_ids[layer] =
            config.sprite_ids.get(index as usize).copied().unwrap_or(0);
    }
}

pub fn tick_sprite_animators(time: Res<Time>, mut query: Query<&mut SpriteAnimator>) {
    for mut animator in &mut query {
        if animator.timer.duration().is_zero() || animator.moving_animation {
            continue;
        }

        // Ticking through the `Mut` would mark the animator `Changed` on every
        // frame, not just the ones where the phase advances. Downstream that is
        // expensive: the `Changed<SpriteAnimator>` filters in the instance
        // update systems stop filtering anything, every write dirties the
        // instance buffer, and the whole SSBO is re-uploaded each frame for as
        // long as a single animated item is on screen. The advance below is the
        // only observable change, so it is the only one that gets flagged.
        let inner = animator.bypass_change_detection();
        inner.timer.tick(time.delta());
        if !inner.timer.just_finished() {
            continue;
        }

        let phase_count = inner.config.animation.total_animation_phases();
        inner.current_phase = (inner.current_phase + 1) % phase_count;
        resolve_simple_sprite_ids(inner);
        animator.set_changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    const PHASE: Duration = Duration::from_millis(100);

    /// Two phases, one layer, no patterns: sprite id is just the phase index.
    fn two_phase_config() -> Arc<SpriteConfig> {
        Arc::new(SpriteConfig {
            id: 1,
            group: "test".to_string(),
            pattern_x: 1,
            pattern_y: 1,
            pattern_z: 1,
            layers: 1,
            sprite_ids: vec![10, 20],
            animation: SpriteAnimation::Uniform {
                phase_count: 2,
                phase_duration: PHASE,
            },
            boxes: Vec::new(),
            shift: Vec2::ZERO,
        })
    }

    fn spawn_animator(world: &mut World) -> Entity {
        world.insert_resource(Time::<()>::default());
        let entity = world
            .spawn(SpriteAnimator::new(two_phase_config(), 0, 0, 0))
            .id();
        world.clear_trackers();
        entity
    }

    /// Runs one frame and reports the animator's change tick and sprite id.
    fn tick(world: &mut World, entity: Entity, delta: Duration) -> (u32, u32) {
        world.resource_mut::<Time<()>>().advance_by(delta);
        world.run_system_once(tick_sprite_animators).unwrap();
        (changed_tick(world, entity), sprite_id(world, entity))
    }

    fn changed_tick(world: &World, entity: Entity) -> u32 {
        world
            .entity(entity)
            .get_change_ticks::<SpriteAnimator>()
            .unwrap()
            .changed
            .get()
    }

    fn sprite_id(world: &World, entity: Entity) -> u32 {
        world
            .entity(entity)
            .get::<SpriteAnimator>()
            .unwrap()
            .current_sprite_ids[0]
    }

    /// The whole point: a frame that only advances the timer must not mark the
    /// animator changed, or the `Changed<SpriteAnimator>` filters downstream
    /// match everything and the instance buffer is re-uploaded every frame.
    #[test]
    fn a_tick_without_a_phase_advance_is_not_a_change() {
        let mut world = World::new();
        let entity = spawn_animator(&mut world);
        let before = changed_tick(&world, entity);

        let (changed, sprite_id) = tick(&mut world, entity, PHASE / 10);

        assert_eq!(changed, before, "no phase advance, so no change");
        assert_eq!(sprite_id, 10, "still on the first phase");
    }

    /// …and the bypass must not swallow the elapsed time: the partial ticks
    /// still accumulate, and the frame that completes the phase does report a
    /// change.
    #[test]
    fn the_phase_advance_accumulates_and_is_reported_as_a_change() {
        let mut world = World::new();
        let entity = spawn_animator(&mut world);
        let before = changed_tick(&world, entity);

        for _ in 0..4 {
            let (changed, sprite_id) = tick(&mut world, entity, PHASE / 5);
            assert_eq!(changed, before, "still short of a full phase");
            assert_eq!(sprite_id, 10);
        }

        let (changed, sprite_id) = tick(&mut world, entity, PHASE / 5);
        assert_ne!(changed, before, "the phase advanced");
        assert_eq!(sprite_id, 20, "and the sprite followed it");
    }

    #[test]
    fn a_static_animator_never_changes() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let config = Arc::new(SpriteConfig {
            id: 1,
            group: "test".to_string(),
            pattern_x: 1,
            pattern_y: 1,
            pattern_z: 1,
            layers: 1,
            sprite_ids: vec![10],
            animation: SpriteAnimation::Static,
            boxes: Vec::new(),
            shift: Vec2::ZERO,
        });
        let entity = world.spawn(SpriteAnimator::new(config, 0, 0, 0)).id();
        world.clear_trackers();
        let before = changed_tick(&world, entity);

        let (changed, _) = tick(&mut world, entity, PHASE * 10);

        assert_eq!(changed, before);
    }
}
