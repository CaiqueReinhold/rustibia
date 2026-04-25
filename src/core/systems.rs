use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::network::{SendMessage, events::ServerPong};

#[derive(Resource, Debug)]
pub struct PingState {
    pub last_sent_at: Instant,
    pub current_ping: Duration,
    timer: Timer,
    pending_ack: bool,
}

impl Default for PingState {
    fn default() -> Self {
        PingState {
            timer: Timer::new(Duration::from_secs(2), TimerMode::Repeating),
            last_sent_at: Instant::now(),
            current_ping: Duration::new(0, 0),
            pending_ack: false,
        }
    }
}

pub fn send_ping(mut commands: Commands, mut ping_state: ResMut<PingState>, time: Res<Time>) {
    ping_state.timer.tick(time.delta());
    if ping_state.timer.just_finished() && !ping_state.pending_ack {
        commands.trigger(SendMessage(crate::network::ClientMessage::Ping));
        ping_state.last_sent_at = Instant::now();
        ping_state.pending_ack = true;
    }
}

pub fn receive_pong(_: On<ServerPong>, mut ping_state: ResMut<PingState>) {
    let received_at = Instant::now();
    ping_state.current_ping = received_at - ping_state.last_sent_at;
    ping_state.pending_ack = false;
}
