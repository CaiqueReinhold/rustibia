use bevy::prelude::*;

use crate::{
    actor::Player,
    camera::GameCamera,
    items::{Item, ItemDragEnded, ItemDragStarted},
    main_ui::GameViewport,
    map::{Map, TileChanged, TilePosition},
};

#[derive(Resource, Debug, Default)]
pub struct MouseHoverState {
    pub screen_position: Vec2,
    pub world_position: Option<Vec2>,
    pub tile_position: Option<TilePosition>,
}

#[derive(Resource, Debug)]
pub struct ItemDragState {
    item: Item,
    origin_position: TilePosition,
}

pub fn update_hover_state(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<GameCamera>>,
    player_position: Single<&TilePosition, With<Player>>,
    mut hover_state: ResMut<MouseHoverState>,
) {
    let Some(mouse_position) = window.cursor_position() else {
        return;
    };
    hover_state.screen_position = mouse_position;

    let (camera, camera_transform) = *camera;
    let Some(viewport) = &camera.viewport else {
        return;
    };

    let viewport_rect = Rect::from_corners(
        viewport.physical_position.as_vec2(),
        (viewport.physical_position + viewport.physical_size).as_vec2(),
    );

    if viewport_rect.contains(mouse_position) {
        let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, mouse_position) else {
            return;
        };
        hover_state.world_position = Some(world_pos);
        hover_state.tile_position =
            Some(TilePosition::from_world(world_pos, player_position.floor));
    } else {
        hover_state.tile_position = None;
    }
}

pub fn attach_observers(mut commands: Commands, overlay_q: Query<Entity, With<GameViewport>>) {
    let entity = overlay_q.single().unwrap();
    commands
        .entity(entity)
        .observe(on_drag_start)
        .observe(on_drag_end)
        .observe(on_tile_click);
}

fn on_drag_start(
    _: On<Pointer<DragStart>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
) {
    let Some(position) = &hover_state.tile_position else {
        return;
    };

    let Some(item) = map.peek_item(position) else {
        return;
    };

    if !item.config.can_move {
        return;
    }

    commands.insert_resource(ItemDragState {
        item: item.clone(),
        origin_position: position.clone(),
    });
    commands.trigger(ItemDragStarted {
        item: item.clone(),
        origin: crate::items::ItemDragOrigin::Map {
            position: position.clone(),
        },
    });
}

fn on_drag_end(
    _: On<Pointer<DragEnd>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    drag_state: Res<ItemDragState>,
    mut map: ResMut<Map>,
) {
    info!("drag ended");

    let canceled = hover_state
        .tile_position
        .as_ref()
        .is_none_or(|target_position| {
            map.remove_item(&drag_state.item, &drag_state.origin_position)
                .and_then(|_| map.add_item(drag_state.item.clone(), target_position))
                .is_err()
        });

    if !canceled {
        let target_pos = hover_state.tile_position.as_ref().unwrap();
        commands.trigger(TileChanged {
            position: target_pos.clone(),
        });
        commands.trigger(TileChanged {
            position: drag_state.origin_position.clone(),
        });
    }

    commands.trigger(ItemDragEnded { canceled });
    commands.remove_resource::<ItemDragState>();
}

fn on_tile_click(event: On<Pointer<Click>>) {
    if event.button == PointerButton::Secondary {}
}
