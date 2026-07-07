use bevy::prelude::*;

use crate::{
    core::TextMessageType,
    game_ui::WindowId,
    items::{ItemDragEnded, ItemId, ItemMoveCanceled, ItemMoveConfirmed, ItemPlacement},
    map::{Position, minimap::MinimapData},
    network::{
        ClientMessage, SendMessage,
        events::{MoveItemResult, ShowTextMessage},
    },
    player::{
        components::Player,
        movement::MovementQueue,
        pathfinding::{
            AutoWalkTarget, compute_path, compute_path_adjacent_to_both, compute_path_to_adjacent,
            is_adjacent,
        },
    },
};

use super::mode::{InteractionMode, PendingLook, PendingUseAck};

/// A semantic player action, produced by gesture handlers and consumed by
/// `on_interaction_intent`. Encoding to `ClientMessage` happens at send time,
/// so a deferred intent is still valid after the walk completes.
#[derive(Event, Debug, Clone)]
pub enum InteractionIntent {
    WalkTo(Position),
    Look(Position),
    MoveItem {
        origin: ItemPlacement,
        item_id: ItemId,
        amount: u8,
        to: ItemPlacement,
    },
    UseItem {
        target: ItemPlacement,
        item_id: ItemId,
        /// Container window to reuse when the server ack opens a container.
        window_id: Option<WindowId>,
    },
    UseItemWith {
        source: ItemPlacement,
        source_item_id: ItemId,
        target: ItemPlacement,
        target_item_id: ItemId,
    },
}

impl InteractionIntent {
    fn to_message(&self) -> Option<ClientMessage> {
        match self {
            InteractionIntent::WalkTo(_) => None,
            InteractionIntent::Look(position) => Some(ClientMessage::Look {
                position: position.clone(),
            }),
            InteractionIntent::MoveItem {
                origin,
                item_id,
                amount,
                to,
            } => Some(ClientMessage::MoveItem {
                from: origin.to_wire_position(),
                item_id: *item_id,
                amount: *amount,
                stack_index: origin.wire_stack_index(),
                to: to.to_wire_position(),
            }),
            InteractionIntent::UseItem {
                target, item_id, ..
            } => Some(ClientMessage::UseItem {
                position: target.to_wire_position(),
                item_id: *item_id,
                stack_index: target.wire_stack_index(),
            }),
            InteractionIntent::UseItemWith {
                source,
                source_item_id,
                target,
                target_item_id,
            } => Some(ClientMessage::UseItemWith {
                source: source.to_wire_position(),
                source_item_id: *source_item_id,
                source_index: source.wire_stack_index(),
                target: target.to_wire_position(),
                target_item_id: *target_item_id,
                target_index: target.wire_stack_index(),
            }),
        }
    }

    /// Map-floor tiles the player must stand on or next to before this
    /// intent may be sent. At most two entries (use-with source + target).
    fn required_map_positions(&self) -> Vec<Position> {
        let mut positions = Vec::new();
        let mut push_if_map = |placement: &ItemPlacement| {
            if let ItemPlacement::Map { position, .. } = placement {
                positions.push(position.clone());
            }
        };
        match self {
            InteractionIntent::WalkTo(_) | InteractionIntent::Look(_) => {}
            InteractionIntent::MoveItem { origin, .. } => push_if_map(origin),
            InteractionIntent::UseItem { target, .. } => push_if_map(target),
            InteractionIntent::UseItemWith { source, target, .. } => {
                push_if_map(source);
                push_if_map(target);
            }
        }
        positions
    }
}

#[derive(Resource, Debug)]
pub struct PendingWalkAction {
    pub item_pos: Position,
    pub intent: InteractionIntent,
}

/// Encode and send an intent, plus its post-send bookkeeping. Called both by
/// the dispatcher (when already in reach) and by `fire_pending_action` (after
/// a deferred walk completes).
pub fn send_intent(commands: &mut Commands, intent: &InteractionIntent) {
    let Some(msg) = intent.to_message() else {
        return;
    };
    commands.trigger(SendMessage(msg));
    match intent {
        InteractionIntent::WalkTo(_) => {}
        InteractionIntent::Look(_) => {
            commands.insert_resource(PendingLook);
        }
        InteractionIntent::MoveItem { .. } => {
            commands.trigger(ItemDragEnded);
        }
        InteractionIntent::UseItem { window_id, .. } => {
            commands.insert_resource(PendingUseAck {
                target_window_id: *window_id,
            });
        }
        InteractionIntent::UseItemWith { .. } => {
            commands.insert_resource(PendingUseAck {
                target_window_id: None,
            });
        }
    }
}

fn show_no_way(commands: &mut Commands) {
    commands.trigger(ShowTextMessage {
        text: "There is no way.".to_string(),
        message_type: TextMessageType::ActionDenied,
    });
}

pub fn on_interaction_intent(
    event: On<InteractionIntent>,
    mut commands: Commands,
    minimap: Res<MinimapData>,
    mut move_queue: ResMut<MovementQueue>,
    mut mode: ResMut<InteractionMode>,
    player_q: Single<&Position, With<Player>>,
) {
    let player_pos = player_q.into_inner();
    let intent = &*event;

    if let InteractionIntent::WalkTo(target) = intent {
        match compute_path(player_pos, target, &minimap) {
            Some(steps) => {
                move_queue.set_auto_walk_path(steps);
                commands.insert_resource(AutoWalkTarget(target.clone()));
            }
            None => show_no_way(&mut commands),
        }
        return;
    }

    let reaches = |p: &Position| player_pos == p || is_adjacent(player_pos, p);
    let required = intent.required_map_positions();

    if required.iter().all(reaches) {
        send_intent(&mut commands, intent);
        return;
    }

    // Deferring a move-item: the visual drag ends now, the message fires
    // on arrival.
    if matches!(intent, InteractionIntent::MoveItem { .. }) {
        if mode.is_dragging() {
            *mode = InteractionMode::Idle;
        }
        commands.trigger(ItemMoveCanceled);
    }

    let path = match required.as_slice() {
        [single] => compute_path_to_adjacent(player_pos, single, &minimap),
        [a, b] => compute_path_adjacent_to_both(player_pos, a, b, &minimap),
        _ => None,
    };
    // Anchor on the last required position: the use-with target / the item
    // being moved or used. Matches the pre-refactor behavior.
    let anchor = required
        .last()
        .expect("unreached position implies at least one required position")
        .clone();

    match path {
        Some(steps) => {
            move_queue.set_auto_walk_path(steps);
            commands.insert_resource(AutoWalkTarget(anchor.clone()));
            commands.insert_resource(PendingWalkAction {
                item_pos: anchor,
                intent: intent.clone(),
            });
        }
        None => show_no_way(&mut commands),
    }
}

pub fn on_move_item_result(
    event: On<MoveItemResult>,
    mut commands: Commands,
    mut mode: ResMut<InteractionMode>,
) {
    if mode.is_dragging() {
        *mode = InteractionMode::Idle;
    }
    if event.success {
        commands.trigger(ItemMoveConfirmed);
    } else {
        commands.trigger(ItemMoveCanceled);
    }
}
