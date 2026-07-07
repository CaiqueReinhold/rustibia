use std::sync::Arc;

use bevy::prelude::*;
use bevy::window::CursorIcon;

use crate::{
    game_ui::WindowId,
    items::{Item, ItemId, ItemPlacement},
    network::events::UseItemAck,
};

#[derive(Resource, Debug, Default)]
pub enum InteractionMode {
    #[default]
    Idle,
    Dragging {
        item: Arc<Item>,
        origin: ItemPlacement,
        crossed_threshold: bool,
    },
    Targeting {
        source: ItemPlacement,
        source_item_id: ItemId,
    },
}

impl InteractionMode {
    pub fn is_dragging(&self) -> bool {
        matches!(self, InteractionMode::Dragging { .. })
    }

    pub fn drag_crossed_threshold(&self) -> bool {
        matches!(
            self,
            InteractionMode::Dragging {
                crossed_threshold: true,
                ..
            }
        )
    }

    pub fn is_targeting(&self) -> bool {
        matches!(self, InteractionMode::Targeting { .. })
    }

    fn clear_targeting_if_gone(&mut self, gone: impl FnOnce(&ItemPlacement, ItemId) -> bool) {
        if let InteractionMode::Targeting {
            source,
            source_item_id,
        } = &*self
            && gone(source, *source_item_id)
        {
            *self = InteractionMode::Idle;
        }
    }
}

#[derive(Resource, Debug)]
pub struct PendingUseAck {
    pub target_window_id: Option<WindowId>,
}

#[derive(Resource, Debug)]
pub struct PendingLook;

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
    mode: Res<InteractionMode>,
    window_q: Single<Entity, With<Window>>,
    mut last_active: Local<bool>,
) {
    let active = mode.is_targeting();
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

pub fn on_targeting_tile_changed(
    event: On<crate::network::events::TileChanged>,
    mut mode: ResMut<InteractionMode>,
) {
    mode.clear_targeting_if_gone(|source, item_id| {
        let ItemPlacement::Map { position, index } = source else {
            return false;
        };
        if position != &event.position {
            return false;
        }
        !event
            .items
            .iter()
            .filter_map(|opt| opt.as_ref())
            .nth(*index)
            .map(|(id, _)| *id == item_id)
            .unwrap_or(false)
    });
}

pub fn on_targeting_container_updated(
    event: On<crate::network::events::UpdateContainer>,
    mut mode: ResMut<InteractionMode>,
) {
    mode.clear_targeting_if_gone(|source, item_id| {
        let ItemPlacement::Container { container_id, slot } = source else {
            return false;
        };
        if *container_id != event.container_id {
            return false;
        }
        !event
            .items
            .get(*slot)
            .and_then(|opt| opt.as_ref())
            .map(|(id, _)| *id == item_id)
            .unwrap_or(false)
    });
}

pub fn on_targeting_container_closed(
    event: On<crate::network::events::ContainerClosed>,
    mut mode: ResMut<InteractionMode>,
) {
    mode.clear_targeting_if_gone(|source, _| {
        matches!(source, ItemPlacement::Container { container_id, .. }
            if *container_id == event.container_id)
    });
}

pub fn on_targeting_inventory_updated(
    event: On<crate::network::events::IventorySlotUpdated>,
    mut mode: ResMut<InteractionMode>,
) {
    mode.clear_targeting_if_gone(|source, item_id| {
        let ItemPlacement::Inventory { slot } = source else {
            return false;
        };
        *slot == event.slot && !event.item_id.map(|id| id == item_id).unwrap_or(false)
    });
}
