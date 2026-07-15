use bevy::prelude::*;

use crate::{
    conf::ui::MIN_DRAG_THRESHOLD,
    game_ui::{MainUI, UiWindowRef},
    items::{ItemDragEnded, ItemDragStarted, ItemFlag, ItemPlacement, LootContainerUI},
    map::Map,
    player::components::PlayerInventory,
};

use super::hover::{MapPick, MouseHoverState, cursor_target, valid_drop_target};
use super::intent::InteractionIntent;
use super::mode::InteractionMode;

pub fn attach_observers(event: On<Add, MainUI>, mut commands: Commands) {
    commands
        .entity(event.entity)
        .observe(on_drag_start)
        .observe(on_drag)
        .observe(on_drag_end)
        .observe(on_click);
}

fn on_drag(event: On<Pointer<Drag>>, mut commands: Commands, mut mode: ResMut<InteractionMode>) {
    let InteractionMode::Dragging {
        item,
        origin,
        crossed_threshold,
    } = &mut *mode
    else {
        return;
    };

    if *crossed_threshold || event.distance.max_element().abs() < MIN_DRAG_THRESHOLD {
        return;
    }

    *crossed_threshold = true;
    commands.trigger(ItemDragStarted {
        item: item.clone(),
        origin: origin.clone(),
    });
}

fn on_drag_start(
    event: On<Pointer<DragStart>>,
    mut mode: ResMut<InteractionMode>,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    container_q: Query<(&LootContainerUI, &UiWindowRef)>,
    inventory: Res<PlayerInventory>,
) {
    if mode.is_targeting() {
        return; // targeting owns the pointer; Escape or a click ends it
    }
    *mode = InteractionMode::Idle;
    if event.button != PointerButton::Primary {
        return;
    }

    let Some(target) = cursor_target(&hover_state, &map, &container_q, &inventory, MapPick::Top)
    else {
        return;
    };

    if target.item.config.has_flag(ItemFlag::Unmove) {
        return;
    }

    *mode = InteractionMode::Dragging {
        item: target.item,
        origin: target.placement,
        crossed_threshold: false,
    };
}

fn on_drag_end(
    _: On<Pointer<DragEnd>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    mut mode: ResMut<InteractionMode>,
    map: Res<Map>,
    container_q: Query<(&LootContainerUI, &UiWindowRef)>,
) {
    let InteractionMode::Dragging {
        item,
        origin,
        crossed_threshold,
    } = &*mode
    else {
        return;
    };

    commands.trigger(ItemDragEnded);
    if !crossed_threshold {
        *mode = InteractionMode::Idle;
        return;
    }

    let (item, origin) = (item.clone(), origin.clone());
    *mode = InteractionMode::Idle;

    let Some(to) = valid_drop_target(&item, &hover_state, &map, &container_q) else {
        return;
    };

    if to.to_wire_position() == origin.to_wire_position() {
        return;
    }

    commands.trigger(InteractionIntent::MoveItem {
        origin,
        item_id: item.config.id,
        amount: item.amount as u8,
        to,
    });
}

fn on_click(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    mut mode: ResMut<InteractionMode>,
    container_q: Query<(&LootContainerUI, &UiWindowRef)>,
    inventory: Res<PlayerInventory>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Targeting owns the click: resolve UseItemWith on primary, cancel on
    // secondary.
    if let InteractionMode::Targeting {
        source,
        source_item_id,
    } = &*mode
    {
        let (source, source_item_id) = (source.clone(), *source_item_id);
        *mode = InteractionMode::Idle;

        if event.button != PointerButton::Primary {
            return;
        }

        let Some(target) = cursor_target(
            &hover_state,
            &map,
            &container_q,
            &inventory,
            MapPick::PreferForceUse,
        ) else {
            return;
        };

        commands.trigger(InteractionIntent::UseItemWith {
            source,
            source_item_id,
            target: target.placement,
            target_item_id: target.item.config.id,
        });
        return;
    }

    if mode.drag_crossed_threshold() {
        return;
    }

    if event.button == PointerButton::Primary {
        // Shift+primary → look.
        if keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            if let Some(target) =
                cursor_target(&hover_state, &map, &container_q, &inventory, MapPick::Top)
            {
                commands.trigger(InteractionIntent::Look(target.placement));
            }
            return;
        }

        // Unmodified primary → walk.
        if let Some(target) = &hover_state.tile_position
            && !keyboard.any_pressed([
                KeyCode::ControlLeft,
                KeyCode::ControlRight,
                KeyCode::AltLeft,
                KeyCode::AltRight,
            ])
        {
            commands.trigger(InteractionIntent::WalkTo(target.clone()));
        }
        return;
    }

    if event.button == PointerButton::Secondary {
        let Some(target) =
            cursor_target(&hover_state, &map, &container_q, &inventory, MapPick::Top)
        else {
            return;
        };

        if target.item.config.has_flag(ItemFlag::MultiUse) {
            *mode = InteractionMode::Targeting {
                source: target.placement.clone(),
                source_item_id: target.item.config.id,
            };
            return;
        }

        if !target.item.config.has_flag(ItemFlag::Usable) {
            return;
        }

        let window_id = if matches!(target.placement, ItemPlacement::Container { .. })
            && target.item.config.has_flag(ItemFlag::Container)
        {
            target.window_id
        } else {
            None
        };
        commands.trigger(InteractionIntent::UseItem {
            target: target.placement,
            item_id: target.item.config.id,
            window_id,
        });
    }
}
