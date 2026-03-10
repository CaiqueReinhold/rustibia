use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    actor::Player,
    camera::GameCamera,
    items::{Item, ItemDragEnded, ItemDragStarted, LootContainerUI, OpenContainer},
    main_ui::MainUI,
    map::{Map, TileChanged, TilePosition},
};

#[derive(Clone, Debug)]
pub enum ItemDragOrigin {
    Map {
        position: TilePosition,
        index: usize,
    },
    Container {
        container: Entity,
        slot: usize,
    },
}

#[derive(Resource, Debug)]
pub struct ItemDragState {
    item: Arc<Item>,
    origin: ItemDragOrigin,
}

#[derive(Resource, Debug, Default)]
pub struct MouseHoverState {
    pub screen_position: Vec2,
    // pub world_position: Option<Vec2>,
    pub tile_position: Option<TilePosition>,
    pub container: Option<Entity>,
    pub container_slot: Option<usize>,
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
        // hover_state.world_position = Some(world_pos);
        hover_state.tile_position =
            Some(TilePosition::from_world(world_pos, player_position.floor));
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

        if !item.config.can_move {
            return;
        }

        commands.insert_resource(ItemDragState {
            item: item.clone(),
            origin: ItemDragOrigin::Map {
                position: position.clone(),
                index,
            },
        });
        commands.trigger(ItemDragStarted {
            item: item.clone(),
            origin: ItemDragOrigin::Map {
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
            origin: ItemDragOrigin::Container { container, slot },
        });
        commands.trigger(ItemDragStarted {
            item: item.clone(),
            origin: ItemDragOrigin::Container { container, slot },
        });
    }
}

fn on_drag_end(
    _: On<Pointer<DragEnd>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    drag_state: Option<Res<ItemDragState>>,
    mut map: ResMut<Map>,
    mut container_q: Query<&mut LootContainerUI>,
) {
    info!("map drag ended");

    let Some(drag_state) = drag_state else {
        return;
    };

    let mut canceled = false;
    // target map
    if let Some(target_position) = &hover_state.tile_position {
        if let ItemDragOrigin::Map { position, .. } = &drag_state.origin {
            if target_position == position {
                return;
            }
        }
        canceled = map
            .add_item(drag_state.item.clone(), target_position)
            .is_err();
        commands.trigger(TileChanged {
            position: target_position.clone(),
        });
    }

    // target container
    if let Some(container) = hover_state.container {
        let Ok(mut container_ui) = container_q.get_mut(container) else {
            return;
        };
        container_ui.items.insert(0, drag_state.item.clone());
    }

    // clean up origin
    if !canceled {
        if let ItemDragOrigin::Map {
            ref position,
            index,
        } = drag_state.origin
        {
            map.remove_item(index, position).unwrap();
            commands.trigger(TileChanged {
                position: position.clone(),
            });
        }
        if let ItemDragOrigin::Container { container, slot } = drag_state.origin {
            let Ok(mut container_ui) = container_q.get_mut(container) else {
                return;
            };
            if Some(container) == hover_state.container {
                container_ui.items.remove(slot + 1);
            } else {
                container_ui.items.remove(slot);
            }
        }
    }

    commands.trigger(ItemDragEnded { canceled });
    commands.remove_resource::<ItemDragState>();
}

fn on_tile_click(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    hover_state: Res<MouseHoverState>,
    map: Res<Map>,
    drag_state: Option<Res<ItemDragState>>,
) {
    if drag_state.is_some() {
        return;
    }
    info!("on click");
    if event.button == PointerButton::Secondary {
        info!("secondary");
        let Some(position) = &hover_state.tile_position else {
            return;
        };
        let Some((item, _)) = map.peek_item(position) else {
            return;
        };

        if item.config.is_container {
            commands.trigger(OpenContainer { item: item.clone() });
        }
    }
}
