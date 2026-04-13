use std::{collections::VecDeque, time::Duration};

use bevy::prelude::*;

use crate::{
    actor::{FacingDirection, MoveActor, Moving, WalkingDirection},
    camera::GameCamera,
    conf::map::TILE_SIZE,
    core::{ItemConfigs, TextMessageType},
    items::ChangedTileQueue,
    map::{self, Map, Position},
    network::{
        events::{
            AgentChangedDirection, PlayerPosition, PlayerWalk, PlayerWalkDenied, ShowTextMessage,
        },
        ClientMessage, SendMessage,
    },
    player::components::Player,
};

#[derive(Debug)]
enum Movement {
    Walk(WalkingDirection),
    Turn(FacingDirection),
}

#[derive(Resource, Debug, Default)]
pub struct MovementQueue {
    moves: VecDeque<Movement>,
    pending_walk_ack: Option<WalkingDirection>,
    pending_turn_ack: bool,
    predicted_pos: Option<Position>,
}

#[derive(Event, Debug)]
pub struct MovePlayer {
    pub direction: WalkingDirection,
}

#[derive(Event, Debug)]
pub struct ChangePlayerDirection {
    pub direction: FacingDirection,
}

pub fn on_player_walk(event: On<MovePlayer>, mut queue: ResMut<MovementQueue>) {
    if !queue.moves.is_empty() {
        queue.moves.pop_back();
    }

    queue.moves.push_back(Movement::Walk(event.direction));
}

pub fn on_player_change_direction(
    event: On<ChangePlayerDirection>,
    mut queue: ResMut<MovementQueue>,
) {
    if !queue.moves.is_empty() {
        queue.moves.pop_back();
    }

    queue.moves.push_back(Movement::Turn(event.direction));
}

pub fn process_move_queue(
    mut commands: Commands,
    mut queue: ResMut<MovementQueue>,
    map: Res<Map>,
    player: Single<(&Position, Option<&Moving>, &Player)>,
) {
    let (player_pos, moving, player) = *player;

    if moving.is_some() || queue.pending_walk_ack.is_some() || queue.pending_turn_ack {
        return;
    }

    let direction = queue.moves.pop_front();

    if let Some(movement) = direction {
        match movement {
            Movement::Walk(direction) => {
                let new_position = player_pos.clone() + direction;
                if !map.can_walk(&new_position) {
                    commands.trigger(ShowTextMessage {
                        text: "You can't go there".to_string(),
                        _message_type: TextMessageType::ActionDenied,
                    });
                    return;
                }
                queue.pending_walk_ack = Some(direction);
                queue.predicted_pos = Some(new_position);
                commands.trigger(SendMessage {
                    msg: ClientMessage::MovePlayer { direction },
                });
                commands.trigger(MoveActor { direction });
            }
            Movement::Turn(direction) => {
                queue.pending_turn_ack = true;
                commands.trigger(SendMessage {
                    msg: ClientMessage::ChangeDirection { direction },
                });
                commands.trigger(AgentChangedDirection {
                    agent_id: player.agent_id,
                    facing: direction,
                });
            }
        }
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
    if move_queue.predicted_pos.as_ref() != Some(&event.position) {
        move_queue.moves.clear();
        commands.trigger(SendMessage {
            msg: ClientMessage::GetPlayerPosition,
        });
    }
    let direction = move_queue.pending_walk_ack.unwrap();
    let source_pos = event.position.clone() - direction;
    map::events::on_player_walk_ack(
        &mut tile_queue,
        &mut map,
        &config,
        &source_pos,
        direction,
        &event.tiles,
    );
    move_queue.pending_walk_ack = None;
    move_queue.predicted_pos = None;
}

pub fn on_walk_denied(
    _: On<PlayerWalkDenied>,
    mut move_queue: ResMut<MovementQueue>,
    mut commands: Commands,
    player: Single<(Entity, &Position), With<Player>>,
) {
    move_queue.moves.clear();

    if let Some(direction) = move_queue.pending_walk_ack {
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
    move_queue.pending_walk_ack = None;
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
    queue.pending_walk_ack = None;
    queue.predicted_pos = None;

    let (entity, position, moving) = *player;

    let time = if let Some(moving) = moving {
        moving.timer.duration() - moving.timer.elapsed()
    } else {
        Duration::from_millis(1)
    };

    // just add moving so placement, z ordering and moving state is handled by actor move system
    commands.entity(entity).insert(Moving {
        start: position.clone(),
        end: event.position.clone(),
        timer: Timer::new(time, TimerMode::Once),
    });
}

pub fn player_changed_direction_ack(
    event: On<AgentChangedDirection>,
    player: Single<&Player>,
    mut move_queue: ResMut<MovementQueue>,
) {
    if event.agent_id == player.agent_id {
        move_queue.pending_turn_ack = false;
    }
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
