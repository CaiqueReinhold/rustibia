use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bevy::prelude::*;

use crate::network::SendMessage;

const PING_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Resource, Clone, Default)]
pub struct PingState(Arc<AtomicU64>);

impl PingState {
    /// Called from the IO task once a `Pong` decodes.
    pub fn record(&self, rtt: Duration) {
        self.0.store(rtt.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn current(&self) -> Duration {
        Duration::from_micros(self.0.load(Ordering::Relaxed))
    }

    /// Drops the last reading so a dead connection's latency can't be shown
    /// against the next one.
    pub fn reset(&self) {
        self.0.store(0, Ordering::Relaxed);
    }
}

#[derive(Resource)]
pub struct PingTimer(Timer);

impl Default for PingTimer {
    fn default() -> Self {
        Self(Timer::new(PING_INTERVAL, TimerMode::Repeating))
    }
}

/// Drives the ping *cadence* only — the measurement happens in the IO task, which
/// is why nothing here records a timestamp.
///
/// There is no in-flight guard. A lost `Pong` just leaves the previous reading in
/// place until the next round trip completes, and at one ping every two seconds a
/// second request cannot overtake the first on any link worth measuring.
pub fn send_ping(mut commands: Commands, mut timer: ResMut<PingTimer>, time: Res<Time>) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        commands.trigger(SendMessage(crate::network::ClientMessage::Ping));
    }
}
