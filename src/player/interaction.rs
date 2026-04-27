use std::sync::Arc;

use bevy::prelude::*;
use bevy::window::CursorIcon;

use crate::{
    camera::GameCamera,
    conf::{
        map::{CONTAINER_COORD_FLAG, INVENTORY_COORD_FLAG},
        ui::MIN_DRAG_THRESHOLD,
        viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH},
    },
    core::TextMessageType,
    game_ui::{GameViewport, MainUI, UiWindowRef, WindowId},
    items::{
        InventorySlot, Item, ItemDragEnded, ItemDragStarted, ItemFlag, ItemId, ItemMoveCanceled,
        ItemMoveConfirmed, ItemPlacement, LootContainerUI,
    },
    map::{Map, Position, minimap::MinimapData},
    network::{
        ClientMessage, SendMessage,
        events::{MoveItemResult, ShowTextMessage, UseItemAck},
    },
    player::{
        components::{Player, PlayerInventory},
        movement::MovementQueue,
        pathfinding::{
            AutoWalkTarget, compute_path, compute_path_adjacent_to_both, compute_path_to_adjacent,
            is_adjacent,
        },
    },
};

#[derive(Resource, Debug)]
pub struct ItemDragState {
    pub item: Arc<Item>,
    pub origin: ItemPlacement,
    pub crossed_threshold: bool,
}

#[derive(Resource, Debug)]
pub struct PendingUseAck {
    pub target_window_id: Option<WindowId>,
}

#[derive(Resource, Debug)]
pub struct PendingLook;

#[derive(Resource, Debug)]
pub struct UseWithTargetingState {
    pub source: ItemPlacement,
    pub source_item_id: ItemId,
    pub source_index: u8,
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
    UseItemWith {
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
        .observe(on_drag)
        .observe(on_drag_end)
        .observe(on_item_click)
        .observe(on_target_click)
        .observe(on_look_at);
}

fn on_drag(
    event: On<Pointer<Drag>>,
    mut commands: Commands,
    drag_state: Option<ResMut<ItemDragState>>,
) {
    let Some(mut drag_state) = drag_state else {
        return;
    };

    if drag_state.crossed_threshold || event.distance.max_element() < MIN_DRAG_THRESHOLD {
        return;
    }

    drag_state.crossed_threshold = true;
    commands.trigger(ItemDragStarted {
        item: drag_state.item.clone(),
        origin: drag_state.origin.clone(),
    });
}

fn on_drag_start(
    event: On<Pointer<DragStart>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    container_q: Query<&LootContainerUI>,
    inventory: Res<PlayerInventory>,
) {
    commands.remove_resource::<ItemDragState>();
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
            crossed_threshold: false,
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
            crossed_threshold: false,
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
            crossed_threshold: false,
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

    if !drag_state.crossed_threshold {
        commands.remove_resource::<ItemDragState>();
        return;
    }

    let (from_position, stack_index) = match &drag_state.origin {
        ItemPlacement::Map { position, index } => (position.clone(), *index),
        ItemPlacement::Container { container_id, slot } => (
            Position {
                x: CONTAINER_COORD_FLAG,
                y: *container_id,
                z: *slot as u8,
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
    if let ItemPlacement::Map {
        position: ref source_pos,
        ..
    } = drag_state.origin
    {
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
                                y: container_ui.container_id,
                                z: slot as u8,
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if let Some(slot) = hover_state.inventory_slot {
                if let Some(item_slot) = drag_state.item.config.slot {
                    if item_slot == slot
                        || (item_slot == InventorySlot::BothHands
                            && slot == InventorySlot::LeftHand)
                    {
                        Some(Position {
                            x: INVENTORY_COORD_FLAG,
                            y: slot.as_id(),
                            z: 0,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(to) = to {
                let msg = ClientMessage::MoveItem {
                    from: from_position.clone(),
                    item_id: drag_state.item.config.id,
                    amount: drag_state.item.amount as u8,
                    stack_index: stack_index as u8,
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
                            message_type: TextMessageType::ActionDenied,
                        });
                    }
                }
            } else {
                commands.trigger(ShowTextMessage {
                    text: "You cannot put that item there.".to_string(),
                    message_type: TextMessageType::ActionDenied,
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
            commands.trigger(SendMessage(ClientMessage::MoveItem {
                from: from_position,
                item_id: drag_state.item.config.id,
                amount: drag_state.item.amount as u8,
                stack_index: stack_index as u8,
                to: target_position.clone(),
            }));
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
            commands.trigger(SendMessage(ClientMessage::MoveItem {
                from: from_position,
                item_id: drag_state.item.config.id,
                amount: drag_state.item.amount as u8,
                stack_index: stack_index as u8,
                to: Position {
                    x: CONTAINER_COORD_FLAG,
                    y: container_ui.container_id,
                    z: slot as u8,
                },
            }));
            canceled = false;
        }
    } else if let Some(slot) = hover_state.inventory_slot
        && let Some(item_slot) = drag_state.item.config.slot
        && (item_slot == slot
            || (item_slot == InventorySlot::BothHands && slot == InventorySlot::LeftHand))
    {
        commands.trigger(SendMessage(ClientMessage::MoveItem {
            from: from_position,
            item_id: drag_state.item.config.id,
            amount: drag_state.item.amount as u8,
            stack_index: stack_index as u8,
            to: Position {
                x: INVENTORY_COORD_FLAG,
                y: slot.as_id(),
                z: 0,
            },
        }));
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
    targeting: Option<Res<UseWithTargetingState>>,
    container_q: Query<(&LootContainerUI, &UiWindowRef)>,
    inventory: Res<PlayerInventory>,
    minimap: Res<MinimapData>,
    mut move_queue: ResMut<MovementQueue>,
    player_q: Single<&Position, With<Player>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if drag_state.is_some_and(|state| state.crossed_threshold)
        || pending_ack.is_some()
        || targeting.is_some()
    {
        return;
    }

    if event.button == PointerButton::Primary
        && let Some(target) = &hover_state.tile_position
        && !keyboard.any_pressed([
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::AltLeft,
            KeyCode::AltRight,
        ])
    {
        let from = player_q.into_inner();
        match compute_path(from, target, &minimap) {
            Some(steps) => {
                move_queue.set_auto_walk_path(steps);
                commands.insert_resource(AutoWalkTarget(target.clone()));
            }
            None => {
                commands.trigger(ShowTextMessage {
                    text: "There is no way.".to_string(),
                    message_type: TextMessageType::ActionDenied,
                });
            }
        }
        return;
    }

    if event.button == PointerButton::Secondary {
        if let Some(position) = &hover_state.tile_position {
            let Some((item, index)) = map.peek_item(position) else {
                return;
            };

            if item.config.has_flag(ItemFlag::MultiUse) {
                commands.insert_resource(UseWithTargetingState {
                    source: ItemPlacement::Map {
                        position: position.clone(),
                        index,
                    },
                    source_item_id: item.config.id,
                    source_index: index as u8,
                });
                return;
            }

            if item.config.has_flag(ItemFlag::Usable) {
                let player_pos = player_q.into_inner();
                let msg = ClientMessage::UseItem {
                    position: position.clone(),
                    item_id: item.config.id,
                    stack_index: index as u8,
                };
                if is_adjacent(player_pos, position) || player_pos == position {
                    commands.trigger(SendMessage(msg));
                    commands.insert_resource(PendingUseAck {
                        target_window_id: None,
                    });
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
                                message_type: TextMessageType::ActionDenied,
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

            if item.config.has_flag(ItemFlag::MultiUse) {
                commands.insert_resource(UseWithTargetingState {
                    source: ItemPlacement::Container {
                        container_id: container_ui.container_id,
                        slot,
                    },
                    source_item_id: item.config.id,
                    source_index: 0,
                });
                return;
            }

            if item.config.has_flag(ItemFlag::Usable) {
                commands.trigger(SendMessage(ClientMessage::UseItem {
                    position: Position {
                        x: CONTAINER_COORD_FLAG,
                        y: container_ui.container_id,
                        z: slot as u8,
                    },
                    item_id: item.config.id,
                    stack_index: 0,
                }));

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

            if item.config.has_flag(ItemFlag::MultiUse) {
                commands.insert_resource(UseWithTargetingState {
                    source: ItemPlacement::Inventory { slot: *slot },
                    source_item_id: item.config.id,
                    source_index: 0,
                });
                return;
            }

            if item.config.has_flag(ItemFlag::Usable) {
                commands.trigger(SendMessage(ClientMessage::UseItem {
                    position: Position {
                        x: INVENTORY_COORD_FLAG,
                        y: slot.as_id(),
                        z: 0,
                    },
                    item_id: item.config.id,
                    stack_index: 0,
                }));
                commands.insert_resource(PendingUseAck {
                    target_window_id: None,
                });
            }
        }
    }
}

fn on_look_at(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    hover_state: Res<MouseHoverState>,
    pending: Option<Res<PendingLook>>,
    targeting: Option<Res<UseWithTargetingState>>,
) {
    if targeting.is_some() {
        return;
    }
    if pending.is_none()
        && event.button == PointerButton::Primary
        && keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
        && let Some(position) = &hover_state.tile_position
    {
        info!("on_look_at");
        commands.trigger(SendMessage(ClientMessage::Look {
            position: position.clone(),
        }));
        commands.insert_resource(PendingLook);
    }
}

pub fn on_item_ack(
    _: On<UseItemAck>,
    mut commands: Commands,
    pending_ack: Option<Res<PendingUseAck>>,
) {
    if let Some(ack) = pending_ack
        && ack.target_window_id.is_none()
    {
        commands.remove_resource::<PendingUseAck>();
    }
    // If the ack is for opening a container, the resource will be removed in the container system
}

pub fn sync_targeting_cursor(
    mut commands: Commands,
    targeting: Option<Res<UseWithTargetingState>>,
    window_q: Single<Entity, With<Window>>,
    mut last_active: Local<bool>,
) {
    let active = targeting.is_some();
    if active == *last_active {
        return;
    }
    *last_active = active;
    let icon = if active {
        bevy::window::SystemCursorIcon::Crosshair
    } else {
        bevy::window::SystemCursorIcon::Default
    };
    commands.entity(*window_q).insert(CursorIcon::System(icon));
}

fn on_target_click(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    targeting: Option<Res<UseWithTargetingState>>,
    container_q: Query<&LootContainerUI>,
    inventory: Res<PlayerInventory>,
    minimap: Res<MinimapData>,
    mut move_queue: ResMut<MovementQueue>,
    player_q: Single<&Position, With<Player>>,
) {
    let Some(targeting) = targeting else {
        return;
    };

    if event.button == PointerButton::Secondary {
        commands.remove_resource::<UseWithTargetingState>();
        return;
    }

    if event.button != PointerButton::Primary {
        return;
    }

    let target = resolve_target(&hover_state, &map, &container_q, &inventory);
    let Some((target_placement, target_item_id, target_index)) = target else {
        commands.remove_resource::<UseWithTargetingState>();
        return;
    };

    let source_pos = placement_to_position(&targeting.source);
    let target_pos = placement_to_position(&target_placement);

    let msg = ClientMessage::UseItemWith {
        source: source_pos.clone(),
        source_item_id: targeting.source_item_id,
        source_index: targeting.source_index,
        target: target_pos.clone(),
        target_item_id,
        target_index,
    };

    let source_on_player = matches!(
        targeting.source,
        ItemPlacement::Container { .. } | ItemPlacement::Inventory { .. }
    );
    let target_on_player = matches!(
        target_placement,
        ItemPlacement::Container { .. } | ItemPlacement::Inventory { .. }
    );

    let player_pos = player_q.into_inner();

    match (source_on_player, target_on_player) {
        (true, true) => {
            commands.trigger(SendMessage(msg));
            commands.insert_resource(PendingUseAck {
                target_window_id: None,
            });
            commands.remove_resource::<UseWithTargetingState>();
        }
        (false, true) | (true, false) => {
            let map_pos = if source_on_player {
                target_pos.clone()
            } else {
                source_pos.clone()
            };
            if is_adjacent(player_pos, &map_pos) || player_pos == &map_pos {
                commands.trigger(SendMessage(msg));
                commands.insert_resource(PendingUseAck {
                    target_window_id: None,
                });
                commands.remove_resource::<UseWithTargetingState>();
            } else {
                info!("player pos: {}, map pos: {}", player_pos, map_pos);
                match compute_path_to_adjacent(player_pos, &map_pos, &minimap) {
                    Some(steps) => {
                        move_queue.set_auto_walk_path(steps);
                        commands.insert_resource(AutoWalkTarget(map_pos.clone()));
                        commands.insert_resource(PendingWalkAction {
                            item_pos: map_pos,
                            action: WalkAction::UseItemWith { msg },
                        });
                        commands.remove_resource::<UseWithTargetingState>();
                    }
                    None => {
                        commands.trigger(ShowTextMessage {
                            text: "There is no way.".to_string(),
                            message_type: TextMessageType::ActionDenied,
                        });
                        commands.remove_resource::<UseWithTargetingState>();
                    }
                }
            }
        }
        (false, false) => {
            let reaches = |p: &Position, x: &Position| -> bool { p == x || is_adjacent(p, x) };
            if reaches(player_pos, &source_pos) && reaches(player_pos, &target_pos) {
                commands.trigger(SendMessage(msg));
                commands.insert_resource(PendingUseAck {
                    target_window_id: None,
                });
                commands.remove_resource::<UseWithTargetingState>();
            } else {
                match compute_path_adjacent_to_both(player_pos, &source_pos, &target_pos, &minimap)
                {
                    Some(steps) => {
                        move_queue.set_auto_walk_path(steps);
                        commands.insert_resource(AutoWalkTarget(target_pos.clone()));
                        commands.insert_resource(PendingWalkAction {
                            item_pos: target_pos,
                            action: WalkAction::UseItemWith { msg },
                        });
                        commands.remove_resource::<UseWithTargetingState>();
                    }
                    None => {
                        commands.trigger(ShowTextMessage {
                            text: "There is no way.".to_string(),
                            message_type: TextMessageType::ActionDenied,
                        });
                        commands.remove_resource::<UseWithTargetingState>();
                    }
                }
            }
        }
    }
}

fn resolve_target(
    hover_state: &MouseHoverState,
    map: &Map,
    container_q: &Query<&LootContainerUI>,
    inventory: &PlayerInventory,
) -> Option<(ItemPlacement, ItemId, u8)> {
    if let Some(position) = &hover_state.tile_position {
        info!("hover position: {}", position);
        let items = map.get_items(position)?;
        let mut force_use_pick: Option<(ItemId, usize)> = None;
        let mut top_pick: Option<(ItemId, usize)> = None;
        for (i, item) in items.enumerate() {
            top_pick = Some((item.config.id, i));
            if item.config.has_flag(ItemFlag::ForceUse) && force_use_pick.is_none() {
                force_use_pick = Some((item.config.id, i));
            }
        }
        let (item_id, index) = force_use_pick.or(top_pick)?;
        return Some((
            ItemPlacement::Map {
                position: position.clone(),
                index,
            },
            item_id,
            index as u8,
        ));
    }
    if let Some(container) = hover_state.container {
        let container_ui = container_q.get(container).ok()?;
        let slot = hover_state.container_slot?;
        let item = container_ui.items.get(slot)?;
        return Some((
            ItemPlacement::Container {
                container_id: container_ui.container_id,
                slot,
            },
            item.config.id,
            0,
        ));
    }
    if let Some(slot) = hover_state.inventory_slot {
        let item = inventory.items.get(&slot)?;
        return Some((ItemPlacement::Inventory { slot }, item.config.id, 0));
    }
    None
}

fn placement_to_position(placement: &ItemPlacement) -> Position {
    match placement {
        ItemPlacement::Map { position, .. } => position.clone(),
        ItemPlacement::Container { container_id, slot } => Position {
            x: CONTAINER_COORD_FLAG,
            y: *container_id,
            z: *slot as u8,
        },
        ItemPlacement::Inventory { slot } => Position {
            x: INVENTORY_COORD_FLAG,
            y: slot.as_id(),
            z: 0,
        },
    }
}

pub fn on_targeting_tile_changed(
    event: On<crate::network::events::TileChanged>,
    mut commands: Commands,
    targeting: Option<Res<UseWithTargetingState>>,
) {
    let Some(targeting) = targeting else { return };
    let ItemPlacement::Map { position, index } = &targeting.source else {
        return;
    };
    if position != &event.position {
        return;
    }

    let still_present = event
        .items
        .iter()
        .filter_map(|opt| opt.as_ref())
        .nth(*index)
        .map(|(id, _)| *id == targeting.source_item_id)
        .unwrap_or(false);

    if !still_present {
        commands.remove_resource::<UseWithTargetingState>();
    }
}

pub fn on_targeting_container_updated(
    event: On<crate::network::events::UpdateContainer>,
    mut commands: Commands,
    targeting: Option<Res<UseWithTargetingState>>,
) {
    let Some(targeting) = targeting else { return };
    let ItemPlacement::Container { container_id, slot } = &targeting.source else {
        return;
    };
    if *container_id != event.container_id {
        return;
    }

    let still_present = event
        .items
        .get(*slot)
        .and_then(|opt| opt.as_ref())
        .map(|(id, _)| *id == targeting.source_item_id)
        .unwrap_or(false);

    if !still_present {
        commands.remove_resource::<UseWithTargetingState>();
    }
}

pub fn on_targeting_container_closed(
    event: On<crate::network::events::ContainerClosed>,
    mut commands: Commands,
    targeting: Option<Res<UseWithTargetingState>>,
) {
    let Some(targeting) = targeting else { return };
    let ItemPlacement::Container { container_id, .. } = &targeting.source else {
        return;
    };
    if *container_id == event.container_id {
        commands.remove_resource::<UseWithTargetingState>();
    }
}

pub fn on_targeting_inventory_updated(
    event: On<crate::network::events::IventorySlotUpdated>,
    mut commands: Commands,
    targeting: Option<Res<UseWithTargetingState>>,
) {
    let Some(targeting) = targeting else { return };
    let ItemPlacement::Inventory { slot } = &targeting.source else {
        return;
    };
    if *slot != event.slot {
        return;
    }
    let still_present = event
        .item_id
        .map(|id| id == targeting.source_item_id)
        .unwrap_or(false);
    if !still_present {
        commands.remove_resource::<UseWithTargetingState>();
    }
}
