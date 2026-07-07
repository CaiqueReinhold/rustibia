use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    camera::GameCamera,
    conf::viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH},
    game_ui::{GameViewport, UiWindowRef, WindowId},
    items::{InventorySlot, Item, ItemFlag, ItemPlacement, LootContainerUI},
    map::{Map, Position},
    player::components::{Player, PlayerInventory},
};

#[derive(Resource, Debug, Default)]
pub struct MouseHoverState {
    pub screen_position: Vec2,
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

/// What the mouse is currently over, resolved to a concrete item.
#[derive(Debug, Clone)]
pub(super) struct CursorTarget {
    pub placement: ItemPlacement,
    pub item: Arc<Item>,
    /// The UI window hosting the hovered container, if any.
    pub window_id: Option<WindowId>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum MapPick {
    /// The topmost item on the tile (drag sources, plain use).
    Top,
    /// Prefer the first item flagged `ForceUse`, falling back to the topmost
    /// (use-with targets, e.g. aiming a rope at a hole).
    PreferForceUse,
}

/// Single source of truth for the tile → container → inventory hover cascade.
pub(super) fn cursor_target(
    hover: &MouseHoverState,
    map: &Map,
    container_q: &Query<(&LootContainerUI, &UiWindowRef)>,
    inventory: &PlayerInventory,
    pick: MapPick,
) -> Option<CursorTarget> {
    if let Some(position) = &hover.tile_position {
        let (item, index) = match pick {
            MapPick::Top => {
                let (item, index) = map.peek_item(position)?;
                (item.clone(), index)
            }
            MapPick::PreferForceUse => {
                let mut force_use: Option<usize> = None;
                let mut top: Option<usize> = None;
                for (i, item) in map.get_items(position)?.enumerate() {
                    top = Some(i);
                    if force_use.is_none() && item.config.has_flag(ItemFlag::ForceUse) {
                        force_use = Some(i);
                    }
                }
                let index = force_use.or(top)?;
                (map.item_at(position, index)?.clone(), index)
            }
        };
        return Some(CursorTarget {
            placement: ItemPlacement::Map {
                position: position.clone(),
                index,
            },
            item,
            window_id: None,
        });
    }
    if let Some(container) = hover.container {
        let (container_ui, window_ref) = container_q.get(container).ok()?;
        let slot = hover.container_slot?;
        let item = container_ui.items.get(slot)?.clone();
        return Some(CursorTarget {
            placement: ItemPlacement::Container {
                container_id: container_ui.container_id,
                slot,
            },
            item,
            window_id: Some(window_ref.window_id),
        });
    }
    if let Some(slot) = hover.inventory_slot {
        let item = inventory.items.get(&slot)?.clone();
        return Some(CursorTarget {
            placement: ItemPlacement::Inventory { slot },
            item,
            window_id: None,
        });
    }
    None
}

/// Destination the dragged item may be dropped at, or `None` if the hovered
/// spot can't accept it. The `index` of a returned `Map` placement is
/// meaningless for destinations (the wire format ignores it) and is set to 0.
pub(super) fn valid_drop_target(
    dragged: &Item,
    hover: &MouseHoverState,
    map: &Map,
    container_q: &Query<(&LootContainerUI, &UiWindowRef)>,
) -> Option<ItemPlacement> {
    if let Some(position) = &hover.tile_position {
        return map.can_drop_item(position).then(|| ItemPlacement::Map {
            position: position.clone(),
            index: 0,
        });
    }
    if let Some(container) = hover.container {
        let (container_ui, _) = container_q.get(container).ok()?;
        let slot = hover.container_slot?;
        if container_ui.is_full() {
            return None;
        }
        return Some(ItemPlacement::Container {
            container_id: container_ui.container_id,
            slot,
        });
    }
    if let Some(slot) = hover.inventory_slot {
        let item_slot = dragged.config.slot?;
        if item_slot == slot
            || (item_slot == InventorySlot::BothHands && slot == InventorySlot::LeftHand)
        {
            return Some(ItemPlacement::Inventory { slot });
        }
    }
    None
}
