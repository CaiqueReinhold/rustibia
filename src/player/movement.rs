use std::{collections::VecDeque, time::Duration};

use bevy::prelude::*;

use crate::{
    actor::{MoveActor, Moving, WalkingDirection},
    camera::GameCamera,
    conf::map::TILE_SIZE,
    core::ItemConfigs,
    items::ChangedTileQueue,
    map::{self, Map, Position},
    network::{
        events::{PlayerPosition, PlayerWalk, PlayerWalkDenied},
        ClientMessage, SendMessage,
    },
    player::components::Player,
};

#[derive(Resource, Debug, Default)]
pub struct MovementQueue {
    moves: VecDeque<WalkingDirection>,
    pending_ack: Option<WalkingDirection>,
    predicted_pos: Option<Position>,
}

#[derive(Event, Debug)]
pub struct MovePlayer {
    pub direction: WalkingDirection,
}

pub fn on_player_walk(event: On<MovePlayer>, mut queue: ResMut<MovementQueue>) {
    if queue.moves.len() < 2 && queue.moves.back() != Some(&event.direction) {
        queue.moves.push_back(event.direction);
    }
    info!("move queue: {:?}", queue.moves);
}

pub fn process_move_queue(
    mut commands: Commands,
    mut queue: ResMut<MovementQueue>,
    map: Res<Map>,
    player: Single<(&Position, Option<&Moving>), With<Player>>,
) {
    let (player_pos, moving) = *player;

    if moving.is_some() || queue.pending_ack.is_some() {
        return;
    }

    let direction = queue.moves.pop_front();

    if let Some(direction) = direction {
        let new_position = player_pos.clone() + direction;
        if !map.can_walk(&new_position) {
            warn!("You cannot go there"); // TODO show text
            return;
        }

        info!(
            "move to {:?} from {:?} to {:?}",
            direction, player_pos, new_position
        );
        queue.pending_ack = Some(direction);
        queue.predicted_pos = Some(new_position);
        commands.trigger(SendMessage {
            msg: ClientMessage::MovePlayer { direction },
        });
        commands.trigger(MoveActor { direction });
    }
}

pub fn on_ack_walk(
    event: On<PlayerWalk>,
    mut commands: Commands,
    mut move_queue: ResMut<MovementQueue>,
    mut map: ResMut<Map>,
    mut tile_queue: ResMut<ChangedTileQueue>,
    config: Res<ItemConfigs>,
) {
    info!("walk ack: {:?}", event.event().position);
    info!("predicted pos: {:?}", move_queue.predicted_pos);
    if move_queue.predicted_pos.as_ref() != Some(&event.position) {
        move_queue.moves.clear();
        commands.trigger(SendMessage {
            msg: ClientMessage::GetPlayerPosition,
        });
    }
    let direction = move_queue.pending_ack.unwrap();
    let source_pos = event.position.clone() - direction;
    map::events::on_player_walk_ack(
        &mut tile_queue,
        &mut map,
        &config,
        &source_pos,
        direction,
        &event.tiles,
    );
    move_queue.pending_ack = None;
    move_queue.predicted_pos = None;
}

pub fn on_walk_denied(
    _: On<PlayerWalkDenied>,
    mut move_queue: ResMut<MovementQueue>,
    mut commands: Commands,
    player: Single<(Entity, &Position), With<Player>>,
) {
    move_queue.moves.clear();

    if let Some(direction) = move_queue.pending_ack {
        if let Some(predicted_pos) = &move_queue.predicted_pos {
            let (entity, position) = *player;
            let player_pos = predicted_pos.clone() - direction;
            commands.entity(entity).insert(Moving {
                start: position.clone(),
                end: player_pos,
                timer: Timer::new(Duration::from_millis(1), TimerMode::Once),
            });
        }
    }
    move_queue.pending_ack = None;
    move_queue.predicted_pos = None;
}

pub fn on_player_position(
    event: On<PlayerPosition>,
    mut commands: Commands,
    mut queue: ResMut<MovementQueue>,
    player: Single<(Entity, &Position, Option<&Moving>), With<Player>>,
) {
    // receiving player position message means walk was denied by server
    // or client requested position because was out of sync
    // in any case there's a pending ack
    let pending = queue.pending_ack;
    queue.pending_ack = None;
    queue.predicted_pos = None;

    let (entity, position, moving) = *player;

    let expected_pos = if let Some(direction) = pending {
        event.position.clone() + direction
    } else {
        event.position.clone()
    };
    let time = if let Some(moving) = moving {
        moving.timer.duration() - moving.timer.elapsed()
    } else {
        Duration::from_millis(1)
    };

    // just add moving so placement, z ordering and moving state is handled by actor move system
    commands.entity(entity).insert(Moving {
        start: position.clone(),
        end: expected_pos,
        timer: Timer::new(time, TimerMode::Once),
    });
}

pub fn center_on_player(
    player_q: Single<&Transform, With<Player>>,
    camera_q: Single<&mut Transform, (With<GameCamera>, Without<Player>)>,
) {
    let player_transform = *player_q;
    let mut camera_transform = camera_q;

    let target = player_transform.translation + Vec3::new(TILE_SIZE / 2.0, -(TILE_SIZE / 2.0), 0.0);
    camera_transform.translation = Vec3::new(target.x.round(), target.y.round(), target.z);
}
