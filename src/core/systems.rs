use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bevy::prelude::*;

use crate::network::SendMessage;

const PING_INTERVAL: Duration = Duration::from_secs(2);

/// The last measured round-trip time, written by the IO task and read by the UI.
///
/// Deliberately not measured from Bevy systems. A timestamp taken beside
/// `commands.trigger(SendMessage(Ping))` is stamped when the command is *queued*,
/// and the reply is only observed once `receive_messages` runs in `Update` and the
/// resulting trigger is flushed — so such a reading folds in a command flush on the
/// way out, and a frame boundary plus another flush on the way back. At 60fps that
/// quantises every sample by up to 16ms and turns any frame hitch into a phantom
/// latency spike. Both timestamps are therefore taken inside
/// [`crate::network::PersistentConnection::run`], immediately around the socket
/// write and the frame decode.
///
/// The `Arc<AtomicU64>` is what lets the IO task write it: the task outlives any
/// single system and has no `World` access, so a plain resource would not do.
/// Microseconds, because a healthy LAN round trip is under a millisecond and
/// storing milliseconds would read as a flat zero.
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
