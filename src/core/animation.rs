use bevy::prelude::*;
use std::sync::Arc;
use std::time::Duration;

use crate::core::sprite::{AnimationLoop, SpriteAnimation, SpriteConfig};

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
    loops_completed: u32,
    descending: bool,
    finished: bool,
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
            loops_completed: 0,
            descending: false,
            finished: false,
        };
        resolve_simple_sprite_ids(&mut s);
        s
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    fn advance(&mut self) {
        let phase_count = self.config.animation.total_animation_phases();
        // A one-phase animation has nowhere to advance to. Counted still has to
        // finish, or a one-phase effect would hang around for ever.
        if phase_count <= 1 {
            if let AnimationLoop::Counted { count } = self.config.animation.loop_mode() {
                self.loops_completed += 1;
                self.finished = self.loops_completed >= count;
            }
            return;
        }

        match self.config.animation.loop_mode() {
            AnimationLoop::Infinite => {
                self.current_phase = (self.current_phase + 1) % phase_count;
            }
            AnimationLoop::PingPong => {
                if self.descending {
                    self.current_phase -= 1;
                    self.descending = self.current_phase > 0;
                } else {
                    self.current_phase += 1;
                    self.descending = self.current_phase >= phase_count - 1;
                }
            }
            AnimationLoop::Counted { count } => {
                let next = self.current_phase + 1;
                if next < phase_count {
                    self.current_phase = next;
                    return;
                }
                self.loops_completed += 1;
                if self.loops_completed >= count {
                    self.finished = true;
                } else {
                    self.current_phase = 0;
                }
            }
        }
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
        if animator.timer.duration().is_zero()
            || animator.moving_animation
            || animator.is_finished()
        {
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

        inner.advance();
        resolve_simple_sprite_ids(inner);
        animator.set_changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    const PHASE: Duration = Duration::from_millis(100);

    /// One layer, no patterns, so the sprite id is just the phase index times ten.
    fn config(phase_count: u32, loop_mode: AnimationLoop) -> Arc<SpriteConfig> {
        Arc::new(SpriteConfig {
            id: 1,
            group: "test".to_string(),
            pattern_x: 1,
            pattern_y: 1,
            pattern_z: 1,
            layers: 1,
            sprite_ids: (0..phase_count).map(|p| (p + 1) * 10).collect(),
            animation: SpriteAnimation::Uniform {
                loop_mode,
                phase_count,
                phase_duration: PHASE,
            },
            boxes: Vec::new(),
            shift: Vec2::ZERO,
        })
    }

    /// Two phases, infinite: the shape the change-detection tests were written for.
    fn two_phase_config() -> Arc<SpriteConfig> {
        config(2, AnimationLoop::Infinite)
    }

    /// Drives `advance` directly over `steps` phase advances and reports the phase
    /// after each one, paired with whether the animation had finished by then.
    fn advance_phases(config: Arc<SpriteConfig>, steps: usize) -> Vec<(u32, bool)> {
        let mut animator = SpriteAnimator::new(config, 0, 0, 0);
        (0..steps)
            .map(|_| {
                animator.advance();
                (animator.current_phase, animator.is_finished())
            })
            .collect()
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

    /// Every animated outfit is infinite, so this is the path creatures take:
    /// wrap forever and never report finished.
    #[test]
    fn an_infinite_animation_wraps_and_never_finishes() {
        let phases = advance_phases(config(3, AnimationLoop::Infinite), 7);

        assert_eq!(
            phases,
            vec![
                (1, false),
                (2, false),
                (0, false),
                (1, false),
                (2, false),
                (0, false),
                (1, false)
            ]
        );
    }

    /// A counted animation holds its last phase rather than snapping back, so the
    /// frame a despawn observer sees is the one the artist ended on.
    #[test]
    fn a_counted_animation_stops_on_its_last_phase() {
        let phases = advance_phases(config(3, AnimationLoop::Counted { count: 1 }), 4);

        assert_eq!(
            phases,
            vec![(1, false), (2, false), (2, true), (2, true)],
            "phases 0,1,2 then finished, holding 2"
        );
    }

    /// `count` is a number of runs, not of phases: 186 of the 207 effects are
    /// count 1, but a handful run several times before they are over.
    #[test]
    fn a_counted_animation_runs_once_per_count() {
        let phases = advance_phases(config(2, AnimationLoop::Counted { count: 2 }), 5);

        assert_eq!(
            phases,
            vec![(1, false), (0, false), (1, false), (1, true), (1, true)],
            "0,1 then 0,1 -- two runs -- then finished"
        );
    }

    /// A single-phase counted animation has nowhere to advance to, but it still has
    /// to finish or the effect it belongs to would never be despawned.
    #[test]
    fn a_single_phase_counted_animation_still_finishes() {
        let phases = advance_phases(config(1, AnimationLoop::Counted { count: 1 }), 2);

        assert_eq!(phases, vec![(0, true), (0, true)]);
    }

    /// A single-phase endless animation must not finish, and must not divide by or
    /// modulo its way into a panic.
    #[test]
    fn a_single_phase_endless_animation_holds_still() {
        assert_eq!(
            advance_phases(config(1, AnimationLoop::Infinite), 3),
            vec![(0, false), (0, false), (0, false)]
        );
        assert_eq!(
            advance_phases(config(1, AnimationLoop::PingPong), 3),
            vec![(0, false), (0, false), (0, false)]
        );
    }

    /// Ping-pong reverses at both ends without repeating the endpoint, which is
    /// what separates it from a wrap.
    #[test]
    fn a_pingpong_animation_walks_back_down() {
        let phases = advance_phases(config(4, AnimationLoop::PingPong), 8);

        assert_eq!(
            phases.iter().map(|(p, _)| *p).collect::<Vec<_>>(),
            vec![1, 2, 3, 2, 1, 0, 1, 2]
        );
        assert!(
            phases.iter().all(|(_, finished)| !*finished),
            "ping-pong never ends"
        );
    }

    /// A finished animator stops consuming ticks, so a one-shot effect waiting to
    /// be despawned costs nothing and cannot re-dirty the instance buffer.
    #[test]
    fn a_finished_animator_stops_ticking() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let entity = world
            .spawn(SpriteAnimator::new(
                config(2, AnimationLoop::Counted { count: 1 }),
                0,
                0,
                0,
            ))
            .id();
        world.clear_trackers();

        // Two phase boundaries: one to reach the last phase, one to finish.
        tick(&mut world, entity, PHASE);
        tick(&mut world, entity, PHASE);
        assert!(
            world
                .entity(entity)
                .get::<SpriteAnimator>()
                .unwrap()
                .is_finished()
        );

        world.clear_trackers();
        let before = changed_tick(&world, entity);
        let (changed, _) = tick(&mut world, entity, PHASE * 10);

        assert_eq!(
            changed, before,
            "a finished animator reports no more changes"
        );
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
