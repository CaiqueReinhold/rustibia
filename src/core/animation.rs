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

        animator.timer.tick(time.delta());
        if animator.timer.just_finished() {
            let phase_count = animator.config.animation.total_animation_phases();
            animator.current_phase = (animator.current_phase + 1) % phase_count;
            resolve_simple_sprite_ids(&mut animator);
        }
    }
}
