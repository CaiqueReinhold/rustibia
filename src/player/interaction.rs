use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    camera::GameCamera,
    conf::{
        map::{CONTAINER_COORD_FLAG, INVENTORY_COORD_FLAG},
        viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH},
    },
    core::TextMessageType,
    game_ui::{GameViewport, MainUI, UiWindowRef, WindowId},
    items::{
        InventorySlot, Item, ItemDragEnded, ItemDragStarted, ItemFlag, ItemMoveCanceled,
        ItemMoveConfirmed, ItemPlacement, LootContainerUI,
    },
    map::{minimap::MinimapData, Map, Position},
    network::{
        events::{MoveItemResult, ShowTextMessage, UseItemAck},
        ClientMessage, SendMessage,
    },
    player::{
        components::{Player, PlayerInventory},
        movement::MovementQueue,
        pathfinding::{compute_path, compute_path_to_adjacent, is_adjacent, AutoWalkTarget},
    },
};

#[derive(Resource, Debug)]
pub struct ItemDragState {
    item: Arc<Item>,
    origin: ItemPlacement,
}

#[derive(Resource, Debug)]
pub struct PendingUseAck {
    pub target_window_id: Option<WindowId>,
}

#[derive(Debug)]
pub enum WalkAction {
    UseItem {
        msg: ClientMessage,
        target_window_id: Option<WindowId>,
    },
    MoveItem {
        msg: ClientMessage,
    },
}

#[derive(Resource, Debug)]
pub struct PendingWalkAction {
    pub item_pos: Position,
    pub action: WalkAction,
}

#[derive(Resource, Debug, Default)]
pub struct MouseHoverState {
    pub screen_position: Vec2,
    // pub world_position: Option<Vec2>,
    pub tile_position: Option<Position>,
    pub container: Option<Entity>,
    pub container_slot: Option<usize>,
    pub inventory_slot: Option<InventorySlot>,
}

pub fn update_hover_state(
    window: Single<&Window>,
    camera_transform: Single<&GlobalTransform, With<GameCamera>>,
    player_position: Single<&Position, With<Player>>,
    mut hover_state: ResMut<MouseHoverState>,
    node_q: Query<(&ComputedNode, &UiGlobalTransform), With<GameViewport>>,
) {
    let Some(mouse_position) = window.cursor_position() else {
        return;
    };
    hover_state.screen_position = mouse_position;

    let Ok((computed, ui_transform)) = node_q.single() else {
        return;
    };

    let size = computed.size();
    let top_left = ui_transform.translation - size * 0.5;
    let image_rect = Rect::from_corners(top_left, top_left + size);

    if image_rect.contains(mouse_position) {
        let uv = (mouse_position - top_left) / size;
        let cam_pos = camera_transform.translation().truncate();
        let world_pos = cam_pos
            + Vec2::new(
                (uv.x - 0.5) * GAME_VIEW_WIDTH,
                (0.5 - uv.y) * GAME_VIEW_HEIGHT,
            );
        hover_state.tile_position = Some(Position::from_world(world_pos, player_position.z));
        hover_state.container_slot = None;
    } else {
        hover_state.tile_position = None;
    }
}

pub fn attach_observers(event: On<Add, MainUI>, mut commands: Commands) {
    commands
        .entity(event.entity)
        .observe(on_drag_start)
        .observe(on_drag_end)
        .observe(on_item_click);
}

fn on_drag_start(
    event: On<Pointer<DragStart>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    drag_state: Option<Res<ItemDragState>>,
    container_q: Query<&LootContainerUI>,
    inventory: Res<PlayerInventory>,
) {
    if drag_state.is_some() {
        return;
    }

    if event.button != PointerButton::Primary {
        return;
    }

    if let Some(position) = &hover_state.tile_position {
        let Some((item, index)) = map.peek_item(position) else {
            return;
        };

        if item.config.has_flag(ItemFlag::Unmove) {
            return;
        }

        commands.insert_resource(ItemDragState {
            item: item.clone(),
            origin: ItemPlacement::Map {
                position: position.clone(),
                index,
            },
        });
        commands.trigger(ItemDragStarted {
            item: item.clone(),
            origin: ItemPlacement::Map {
                position: position.clone(),
                index,
            },
        });
    } else if let Some(container) = hover_state.container {
        let Some(slot) = hover_state.container_slot else {
            return;
        };
        let Ok(container_ui) = container_q.get(container) else {
            return;
        };
        let Some(item) = container_ui.items.get(slot) else {
            return;
        };

        commands.insert_resource(ItemDragState {
            item: item.clone(),
            origin: ItemPlacement::Container {
                container_id: container_ui.container_id,
                slot,
            },
        });
        commands.trigger(ItemDragStarted {
            item: item.clone(),
            origin: ItemPlacement::Container {
                container_id: container_ui.container_id,
                slot,
            },
        });
    } else if let Some(inventory_slot) = hover_state.inventory_slot {
        let Some(item) = inventory.items.get(&inventory_slot) else {
            return;
        };

        commands.insert_resource(ItemDragState {
            item: item.clone(),
            origin: ItemPlacement::Inventory {
                slot: inventory_slot,
            },
        });
        commands.trigger(ItemDragStarted {
            item: item.clone(),
            origin: ItemPlacement::Inventory {
                slot: inventory_slot,
            },
        });
    }
}

fn on_drag_end(
    _: On<Pointer<DragEnd>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    drag_state: Option<Res<ItemDragState>>,
    map: Res<Map>,
    container_q: Query<&LootContainerUI>,
    minimap: Res<MinimapData>,
    mut move_queue: ResMut<MovementQueue>,
    player_q: Single<&Position, With<Player>>,
) {
    let Some(drag_state) = drag_state else {
        return;
    };

    let (from_position, stack_index) = match &drag_state.origin {
        ItemPlacement::Map { position, index } => (position.clone(), *index),
        ItemPlacement::Container { container_id, slot } => (
            Position {
                x: CONTAINER_COORD_FLAG,
                y: *container_id as u32,
                z: *slot as u32,
            },
            0,
        ),
        ItemPlacement::Inventory { slot } => (
            Position {
                x: INVENTORY_COORD_FLAG,
                y: slot.as_id(),
                z: 0,
            },
            0,
        ),
    };

    // For map-floor sources, defer the move if the player is not adjacent
    if let ItemPlacement::Map { position: ref source_pos, .. } = drag_state.origin {
        let player_pos = player_q.into_inner();
        if !(is_adjacent(player_pos, source_pos) || player_pos == source_pos) {
            // Cancel visual drag state immediately
            commands.remove_resource::<ItemDragState>();
            commands.trigger(ItemMoveCanceled);

            // Determine the drop destination from hover state (same logic as the existing send below)
            let to = if let Some(target_position) = &hover_state.tile_position {
                if map.can_drop_item(target_position) {
                    Some(target_position.clone())
                } else {
                    None
                }
            } else if let Some(container) = hover_state.container {
                if let Ok(container_ui) = container_q.get(container) {
                    if let Some(slot) = hover_state.container_slot {
                        if !container_ui.is_full() {
                            Some(Position {
                                x: CONTAINER_COORD_FLAG,
                                y: container_ui.container_id as u32,
                                z: slot as u32,
                            })
                        } else { None }
                    } else { None }
                } else { None }
            } else if let Some(slot) = hover_state.inventory_slot {
                if let Some(item_slot) = drag_state.item.config.slot {
                    if item_slot == slot
                        || (item_slot == InventorySlot::BothHands && slot == InventorySlot::LeftHand)
                    {
                        Some(Position {
                            x: INVENTORY_COORD_FLAG,
                            y: slot.as_id(),
                            z: 0,
                        })
                    } else { None }
                } else { None }
            } else {
                None
            };

            if let Some(to) = to {
                let msg = ClientMessage::MoveItem {
                    from: from_position.clone(),
                    item_id: drag_state.item.config.id,
                    amount: drag_state.item.amount as u8,
                    stack_index: stack_index as u16,
                    to,
                };
                match compute_path_to_adjacent(player_pos, source_pos, &minimap) {
                    Some(steps) => {
                        move_queue.set_auto_walk_path(steps);
                        commands.insert_resource(AutoWalkTarget(source_pos.clone()));
                        commands.insert_resource(PendingWalkAction {
                            item_pos: source_pos.clone(),
                            action: WalkAction::MoveItem { msg },
                        });
                    }
                    None => {
                        commands.trigger(ShowTextMessage {
                            text: "There is no way.".to_string(),
                            _message_type: TextMessageType::ActionDenied,
                        });
                    }
                }
            } else {
                commands.trigger(ShowTextMessage {
                    text: "You cannot put that item there.".to_string(),
                    _message_type: TextMessageType::ActionDenied,
                });
            }
            return;
        }
    }

    let mut canceled = true;

    if let Some(target_position) = &hover_state.tile_position {
        if *target_position == from_position {
            return;
        }

        if map.can_drop_item(target_position) {
            commands.trigger(SendMessage {
                msg: ClientMessage::MoveItem {
                    from: from_position,
                    item_id: drag_state.item.config.id,
                    amount: drag_state.item.amount as u8,
                    stack_index: stack_index as u16,
                    to: target_position.clone(),
                },
            });
            canceled = false;
        }
    } else if let Some(container) = hover_state.container {
        let Ok(container_ui) = container_q.get(container) else {
            return;
        };
        let Some(slot) = hover_state.container_slot else {
            return;
        };
        if !container_ui.is_full() {
            commands.trigger(SendMessage {
                msg: ClientMessage::MoveItem {
                    from: from_position,
                    item_id: drag_state.item.config.id,
                    amount: drag_state.item.amount as u8,
                    stack_index: stack_index as u16,
                    to: Position {
                        x: CONTAINER_COORD_FLAG,
                        y: container_ui.container_id as u32,
                        z: slot as u32,
                    },
                },
            });
            canceled = false;
        }
    } else if let Some(slot) = hover_state.inventory_slot {
        if let Some(item_slot) = drag_state.item.config.slot {
            if item_slot == slot
                || (item_slot == InventorySlot::BothHands && slot == InventorySlot::LeftHand)
            {
                commands.trigger(SendMessage {
                    msg: ClientMessage::MoveItem {
                        from: from_position,
                        item_id: drag_state.item.config.id,
                        amount: drag_state.item.amount as u8,
                        stack_index: stack_index as u16,
                        to: Position {
                            x: INVENTORY_COORD_FLAG,
                            y: slot.as_id(),
                            z: 0,
                        },
                    },
                });
            }
        }
    }

    if canceled {
        commands.remove_resource::<ItemDragState>();
        commands.trigger(ItemMoveCanceled);
    } else {
        commands.trigger(ItemDragEnded);
    }
}

pub fn on_move_item_result(event: On<MoveItemResult>, mut commands: Commands) {
    commands.remove_resource::<ItemDragState>();
    if event.success {
        commands.trigger(ItemMoveConfirmed);
    } else {
        commands.trigger(ItemMoveCanceled);
    }
}

fn on_item_click(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    drag_state: Option<Res<ItemDragState>>,
    pending_ack: Option<Res<PendingUseAck>>,
    container_q: Query<(&LootContainerUI, &UiWindowRef)>,
    inventory: Res<PlayerInventory>,
    minimap: Res<MinimapData>,
    mut move_queue: ResMut<MovementQueue>,
    player_q: Single<&Position, With<Player>>,
) {
    if drag_state.is_some() || pending_ack.is_some() {
        return;
    }

    if event.button == PointerButton::Primary {
        if let Some(target) = &hover_state.tile_position {
            let from = player_q.into_inner();
            match compute_path(from, target, &minimap) {
                Some(steps) => {
                    move_queue.set_auto_walk_path(steps);
                    commands.insert_resource(AutoWalkTarget(target.clone()));
                }
                None => {
                    commands.trigger(ShowTextMessage {
                        text: "There is no way.".to_string(),
                        _message_type: TextMessageType::ActionDenied,
                    });
                }
            }
            return;
        }
    }

    if event.button == PointerButton::Secondary {
        if let Some(position) = &hover_state.tile_position {
            let Some((item, index)) = map.peek_item(position) else {
                return;
            };

            if item.config.has_flag(ItemFlag::Usable) {
                let player_pos = player_q.into_inner();
                let msg = ClientMessage::UseItem {
                    position: position.clone(),
                    item_id: item.config.id,
                    stack_index: index as u16,
                };
                if is_adjacent(player_pos, position) || player_pos == position {
                    commands.trigger(SendMessage { msg });
                    commands.insert_resource(PendingUseAck { target_window_id: None });
                } else {
                    match compute_path_to_adjacent(player_pos, position, &minimap) {
                        Some(steps) => {
                            move_queue.set_auto_walk_path(steps);
                            commands.insert_resource(AutoWalkTarget(position.clone()));
                            commands.insert_resource(PendingWalkAction {
                                item_pos: position.clone(),
                                action: WalkAction::UseItem {
                                    msg,
                                    target_window_id: None,
                                },
                            });
                        }
                        None => {
                            commands.trigger(ShowTextMessage {
                                text: "There is no way.".to_string(),
                                _message_type: TextMessageType::ActionDenied,
                            });
                        }
                    }
                }
            }
        } else if let Some(container) = hover_state.container {
            let Ok((container_ui, window_ref)) = container_q.get(container) else {
                return;
            };
            let Some(slot) = hover_state.container_slot else {
                return;
            };
            let Some(item) = container_ui.items.get(slot) else {
                return;
            };

            if item.config.has_flag(ItemFlag::Usable) {
                commands.trigger(SendMessage {
                    msg: ClientMessage::UseItem {
                        position: Position {
                            x: CONTAINER_COORD_FLAG,
                            y: container_ui.container_id as u32,
                            z: slot as u32,
                        },
                        item_id: item.config.id,
                        stack_index: 0,
                    },
                });

                if item.config.has_flag(ItemFlag::Container) {
                    commands.insert_resource(PendingUseAck {
                        target_window_id: Some(window_ref.window_id),
                    });
                } else {
                    commands.insert_resource(PendingUseAck {
                        target_window_id: None,
                    });
                }
            }
        } else if let Some(slot) = &hover_state.inventory_slot {
            let Some(item) = inventory.items.get(slot) else {
                return;
            };

            if item.config.has_flag(ItemFlag::Usable) {
                commands.trigger(SendMessage {
                    msg: ClientMessage::UseItem {
                        position: Position {
                            x: INVENTORY_COORD_FLAG,
                            y: slot.as_id(),
                            z: 0,
                        },
                        item_id: item.config.id,
                        stack_index: 0,
                    },
                });
                commands.insert_resource(PendingUseAck {
                    target_window_id: None,
                });
            }
        }
    }
}

pub fn on_item_ack(
    _: On<UseItemAck>,
    mut commands: Commands,
    pending_ack: Option<Res<PendingUseAck>>,
) {
    if let Some(ack) = pending_ack {
        if ack.target_window_id.is_none() {
            commands.remove_resource::<PendingUseAck>();
        }
        // If the ack is for opening a container, the resource will be removed in the container system
    }
}
