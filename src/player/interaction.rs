use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    camera::GameCamera,
    conf::{
        map::CONTAINER_COORD_FLAG,
        viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH},
    },
    items::{
        ContainerId, Item, ItemDragEnded, ItemDragStarted, ItemFlag, ItemMoveCanceled,
        ItemMoveConfirmed, LootContainerUI,
    },
    main_ui::{GameViewport, MainUI},
    map::{Map, Position},
    network::{
        events::{MoveItemResult, UseItemAck},
        ClientMessage, SendMessage,
    },
    player::components::Player,
};

#[derive(Clone, Debug)]
pub enum ItemPlacement {
    Map {
        position: Position,
        index: usize,
    },
    Container {
        container_id: ContainerId,
        slot: usize,
    },
}

#[derive(Resource, Debug)]
pub struct ItemDragState {
    item: Arc<Item>,
    origin: ItemPlacement,
}

#[derive(Resource, Debug)]
pub struct PendingUseAck;

#[derive(Resource, Debug, Default)]
pub struct MouseHoverState {
    pub screen_position: Vec2,
    // pub world_position: Option<Vec2>,
    pub tile_position: Option<Position>,
    pub container: Option<Entity>,
    pub container_slot: Option<usize>,
}

pub fn update_hover_state(
    window: Single<&Window>,
    camera_transform: Single<&GlobalTransform, With<GameCamera>>,
    player_position: Single<&Position, With<Player>>,
    mut hover_state: ResMut<MouseHoverState>,
    viewport_q: Query<&Children, With<GameViewport>>,
    node_q: Query<(&ComputedNode, &UiGlobalTransform), With<ImageNode>>,
) {
    let Some(mouse_position) = window.cursor_position() else {
        return;
    };
    hover_state.screen_position = mouse_position;

    let Ok(children) = viewport_q.single() else {
        return;
    };
    let Some((computed, ui_transform)) = children.iter().find_map(|child| node_q.get(child).ok())
    else {
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
        .observe(on_tile_click);
}

fn on_drag_start(
    event: On<Pointer<DragStart>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    drag_state: Option<Res<ItemDragState>>,
    container_q: Query<&LootContainerUI>,
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
        return;
    }

    if let Some(container) = hover_state.container {
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
    }
}

fn on_drag_end(
    _: On<Pointer<DragEnd>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    drag_state: Option<Res<ItemDragState>>,
    map: Res<Map>,
    container_q: Query<&LootContainerUI>,
) {
    let Some(drag_state) = drag_state else {
        return;
    };

    let (from_position, stack_index) = match &drag_state.origin {
        ItemPlacement::Map { position, index } => (position.clone(), index),
        ItemPlacement::Container { container_id, slot } => (
            Position {
                x: CONTAINER_COORD_FLAG,
                y: *container_id as u32,
                z: *slot as u32,
            },
            slot,
        ),
    };

    let mut canceled = false;

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
                    stack_index: *stack_index as u16,
                    to: target_position.clone(),
                },
            });
        } else {
            canceled = true;
        }
    } else if let Some(container) = hover_state.container {
        let Ok(container_ui) = container_q.get(container) else {
            return;
        };
        // let Some(slot) = hover_state.container_slot else {
        //     return;
        // };
        if !container_ui.is_full() {
            commands.trigger(SendMessage {
                msg: ClientMessage::MoveItem {
                    from: from_position,
                    item_id: drag_state.item.config.id,
                    amount: drag_state.item.amount as u8,
                    stack_index: 0,
                    to: Position {
                        x: CONTAINER_COORD_FLAG,
                        y: container_ui.container_id as u32,
                        z: 0,
                    },
                },
            });
        } else {
            canceled = true;
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

fn on_tile_click(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    drag_state: Option<Res<ItemDragState>>,
    pending_ack: Option<Res<PendingUseAck>>,
) {
    if drag_state.is_some() || pending_ack.is_some() {
        return;
    }

    if event.button == PointerButton::Secondary {
        let Some(position) = &hover_state.tile_position else {
            return;
        };
        let Some((item, index)) = map.peek_item(position) else {
            return;
        };

        if item.config.has_flag(ItemFlag::Usable) {
            commands.trigger(SendMessage {
                msg: ClientMessage::UseItem {
                    position: position.clone(),
                    item_id: item.config.id,
                    stack_index: index as u16,
                },
            });
            commands.insert_resource(PendingUseAck);
        }
    }
}

pub fn on_item_ack(_: On<UseItemAck>, mut commands: Commands) {
    commands.remove_resource::<PendingUseAck>();
}
