use std::sync::Arc;

use bevy::{camera::visibility::RenderLayers, prelude::*};

use crate::{
    conf::ui::{UI_ITEM_SIZE, z_index::DRAGGED_ITEM_UI_Z},
    core::{Appearances, SpriteAnimator},
    items::{Item, ItemDragEnded, ItemDragStarted, ItemPlacement, instancing::ItemState},
    player::MouseHoverState,
};

#[derive(Component, Debug)]
#[allow(dead_code)]
pub struct UiItem {
    pub item: Arc<Item>,
}

#[derive(Component)]
pub struct UiItemDragging {
    origin: ItemPlacement,
}

pub fn spawn_ui_item(
    item: &Arc<Item>,
    appearances: &Appearances,
    texture_atlases: &mut Assets<TextureAtlasLayout>,
    position: &Vec2,
) -> impl Bundle {
    let config = appearances.get_item(item.config.id);
    let sheet = appearances.get_sheet(&config.group);
    let texture_atlas = TextureAtlasLayout::from_grid(
        sheet.sprite_size.as_uvec2(),
        sheet.grid_size.x as u32,
        sheet.grid_size.y as u32,
        None,
        None,
    );
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    let mut atlas = TextureAtlas::from(texture_atlas_handle);

    let animator = SpriteAnimator::new(Arc::clone(&config), 0, 0, 0);
    atlas.index = animator.current_sprite_ids[0] as usize;

    (
        UiItem { item: item.clone() },
        animator,
        Node {
            width: Val::Px(UI_ITEM_SIZE),
            height: Val::Px(UI_ITEM_SIZE),
            ..default()
        },
        ImageNode::from_atlas_image(sheet.texture().clone(), atlas),
        Transform::from_xyz(position.x, position.y, 0.0),
        RenderLayers::layer(1),
    )
}

pub fn animate_ui_items(
    mut query: Query<(&SpriteAnimator, &mut ImageNode), Changed<SpriteAnimator>>,
) {
    for (animator, mut image_node) in &mut query {
        if let Some(atlas) = &mut image_node.texture_atlas {
            atlas.index = animator.current_sprite_ids[0] as usize;
        }
    }
}

pub fn item_drag_started(
    event: On<ItemDragStarted>,
    mut commands: Commands,
    appearances: Res<Appearances>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    hover_state: Res<MouseHoverState>,
    state: Res<ItemState>,
    stack_item_q: Query<&Children>,
    drag_item_q: Query<Entity, With<UiItemDragging>>,
) {
    for e in drag_item_q {
        commands.entity(e).despawn();
    }

    if let ItemPlacement::Map { position, index } = &event.origin {
        let Some(stack_entity) = state.occupied_tiles.get(position) else {
            return;
        };
        let Ok(stack_items) = stack_item_q.get(*stack_entity) else {
            return;
        };
        let Some(item_entity) = stack_items.get(*index) else {
            return;
        };

        commands.entity(*item_entity).insert(Visibility::Hidden);
    }

    commands
        .spawn(spawn_ui_item(
            &event.item,
            &appearances,
            &mut texture_atlases,
            &hover_state.screen_position,
        ))
        .insert((
            UiItemDragging {
                origin: event.origin.clone(),
            },
            ZIndex(DRAGGED_ITEM_UI_Z),
        ));
}

pub fn item_drag_ended(
    _: On<ItemDragEnded>,
    mut commands: Commands,
    state: Res<ItemState>,
    stack_item_q: Query<&Children>,
    drag_item_q: Query<(Entity, &UiItemDragging)>,
) {
    let Ok((entity, drag_item)) = drag_item_q.single() else {
        return;
    };
    if let ItemPlacement::Map { position, index } = &drag_item.origin {
        let Some(stack_entity) = state.occupied_tiles.get(position) else {
            return;
        };
        let Ok(stack_items) = stack_item_q.get(*stack_entity) else {
            return;
        };
        let Some(item) = stack_items.get(*index) else {
            return;
        };
        commands.entity(*item).insert(Visibility::Visible);
    };
    commands.entity(entity).despawn();
}

pub fn move_dragged_item(
    ui_item_q: Query<&mut UiTransform, With<UiItemDragging>>,
    hover_state: Res<MouseHoverState>,
) {
    for mut item_transform in ui_item_q {
        item_transform.translation = Val2::new(
            Val::Px(hover_state.screen_position.x),
            Val::Px(hover_state.screen_position.y),
        );
    }
}
