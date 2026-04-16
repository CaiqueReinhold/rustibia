use std::{collections::VecDeque, time::Duration};

use bevy::prelude::*;

use crate::{
    actor::{FacingDirection, MoveActor, Moving, UpdateElevation, WalkingDirection},
    camera::GameCamera,
    conf::map::TILE_SIZE,
    core::{ItemConfigs, TextMessageType},
    items::{ChangedTileQueue, ItemDragEnded},
    map::{self, Map, MinimapData, Position},
    network::{
        events::{
            AgentChangedDirection, PlayerPosition, PlayerWalk, PlayerWalkDenied, ShowTextMessage,
        },
        ClientMessage, SendMessage,
    },
    player::{
        components::Player,
        interaction::{PendingUseAck, PendingWalkAction, WalkAction},
        pathfinding::{compute_path, compute_path_to_adjacent, is_adjacent, AutoWalkTarget},
    },
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

impl MovementQueue {
    pub fn set_auto_walk_path(&mut self, directions: impl IntoIterator<Item = WalkingDirection>) {
        self.moves.clear();
        for dir in directions {
            self.moves.push_back(Movement::Walk(dir));
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct PlayerElevation(f32);

#[derive(Event, Debug)]
pub struct MovePlayer {
    pub direction: WalkingDirection,
}

#[derive(Event, Debug)]
pub struct ChangePlayerDirection {
    pub direction: FacingDirection,
}

pub fn on_player_walk(
    event: On<MovePlayer>,
    mut queue: ResMut<MovementQueue>,
    mut commands: Commands,
) {
    queue.moves.clear();
    queue.moves.push_back(Movement::Walk(event.direction));
    commands.remove_resource::<AutoWalkTarget>();
    commands.remove_resource::<PendingWalkAction>();
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
    mut minimap: ResMut<MinimapData>,
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
        &mut commands,
        &mut tile_queue,
        &mut map,
        &config,
        &mut minimap,
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
    auto_walk_target: Option<Res<AutoWalkTarget>>,
    pending_walk_action: Option<Res<PendingWalkAction>>,
    minimap: Res<MinimapData>,
) {
    move_queue.moves.clear();

    let mut confirmed_pos: Option<Position> = None;

    if let Some(direction) = move_queue.pending_walk_ack {
        if let Some(predicted_pos) = &move_queue.predicted_pos {
            let (entity, position) = *player;
            let player_pos = predicted_pos.clone() - direction;
            commands.entity(entity).insert(Moving {
                start: position.clone(),
                end: player_pos.clone(),
                timer: Timer::new(Duration::from_millis(1), TimerMode::Once),
            });
            confirmed_pos = Some(player_pos);
        }
    }
    move_queue.pending_walk_ack = None;
    move_queue.predicted_pos = None;

    // Deferred-action recalculation takes priority
    if let Some(pending) = &pending_walk_action {
        if let Some(from) = &confirmed_pos {
            match compute_path_to_adjacent(from, &pending.item_pos, &minimap) {
                Some(steps) => move_queue.set_auto_walk_path(steps),
                None => {
                    commands.remove_resource::<PendingWalkAction>();
                    commands.remove_resource::<AutoWalkTarget>();
                    commands.trigger(ShowTextMessage {
                        text: "There is no way.".to_string(),
                        _message_type: TextMessageType::ActionDenied,
                    });
                }
            }
        }
        return;
    }

    if let (Some(target), Some(from)) = (auto_walk_target, confirmed_pos) {
        match compute_path(&from, &target.0, &minimap) {
            Some(steps) => move_queue.set_auto_walk_path(steps),
            None => {
                commands.remove_resource::<AutoWalkTarget>();
                commands.trigger(ShowTextMessage {
                    text: "There is no way.".to_string(),
                    _message_type: TextMessageType::ActionDenied,
                });
            }
        }
    }
}

pub fn on_player_position(
    event: On<PlayerPosition>,
    mut commands: Commands,
    mut queue: ResMut<MovementQueue>,
    player: Single<(Entity, &Position, Option<&Moving>), With<Player>>,
) {
    queue.pending_walk_ack = None;
    queue.predicted_pos = None;

    let (entity, position, moving) = *player;

    let time = if let Some(moving) = moving {
        moving.timer.duration() - moving.timer.elapsed()
    } else {
        Duration::from_millis(1)
    };

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

pub fn update_player_elevation(
    player_q: Single<
        (&Position, Option<&Moving>),
        (With<Player>, Or<(Changed<Position>, Changed<Moving>)>),
    >,
    mut player_elevation: ResMut<PlayerElevation>,
    map: Res<Map>,
) {
    let (pos, moving) = *player_q;
    if let Some(moving) = moving {
        let current_elevation = if moving.timer.fraction() > 0.5 {
            map.get_elevation(&moving.end) as f32
        } else {
            map.get_elevation(&moving.start) as f32
        };
        player_elevation.0 = current_elevation;
    } else {
        player_elevation.0 = map.get_elevation(pos) as f32;
    };
}

pub fn on_update_elevation_player(
    event: On<UpdateElevation>,
    player_position: Single<&Position, With<Player>>,
    mut player_elevation: ResMut<PlayerElevation>,
    map: Res<Map>,
) {
    if event.pos == **player_position {
        player_elevation.0 = map.get_elevation(&event.pos) as f32;
    }
}

pub fn center_on_player(
    player_q: Single<&Transform, With<Player>>,
    camera_q: Single<&mut Transform, (With<GameCamera>, Without<Player>)>,
    player_elevation: Res<PlayerElevation>,
) {
    let player_transform = *player_q;
    let mut camera_transform = camera_q;

    let target = player_transform.translation
        + Vec3::new(TILE_SIZE / 2.0, -(TILE_SIZE / 2.0), 0.0)
        + Vec3::new(player_elevation.0, -player_elevation.0, 0.0);
    camera_transform.translation = Vec3::new(target.x.round(), target.y.round(), 1000.0);
}

pub fn fire_pending_action(
    mut commands: Commands,
    queue: Res<MovementQueue>,
    pending: Option<Res<PendingWalkAction>>,
    player_q: Single<&Position, With<Player>>,
) {
    let Some(pending) = pending else { return };

    if !queue.moves.is_empty() || queue.pending_walk_ack.is_some() || queue.pending_turn_ack {
        return;
    }

    let player_pos = *player_q;
    if !(is_adjacent(player_pos, &pending.item_pos) || player_pos == &pending.item_pos) {
        return;
    }

    match &pending.action {
        WalkAction::UseItem {
            msg,
            target_window_id,
        } => {
            commands.trigger(SendMessage { msg: msg.clone() });
            commands.insert_resource(PendingUseAck {
                target_window_id: *target_window_id,
            });
        }
        WalkAction::MoveItem { msg } => {
            commands.trigger(SendMessage { msg: msg.clone() });
            commands.trigger(ItemDragEnded);
        }
    }

    commands.remove_resource::<PendingWalkAction>();
    commands.remove_resource::<AutoWalkTarget>();
}
