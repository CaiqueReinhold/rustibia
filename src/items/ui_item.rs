use std::sync::Arc;

use bevy::{camera::visibility::RenderLayers, prelude::*};

use crate::{
    conf::ui::{z_index::DRAGGED_ITEM_UI_Z, UI_ITEM_SIZE},
    core::Appearances,
    items::{instancing::ItemStacks, Item},
    player::{ItemPlacement, MouseHoverState},
};

#[derive(Event)]
pub struct ItemDragStarted {
    pub item: Arc<Item>,
    pub origin: ItemPlacement,
}

#[derive(Event)]
pub struct ItemDragEnded;

#[derive(Event)]
pub struct ItemMoveCanceled;

#[derive(Event)]
pub struct ItemMoveConfirmed;

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
        UVec2::splat(if config.box_size > 32.0 { 64 } else { 32 }),
        sheet.grid_size.x as u32,
        sheet.grid_size.y as u32,
        None,
        None,
    );
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    let mut atlas = TextureAtlas::from(texture_atlas_handle);
    atlas.index = (*config.sprite_ids.first().unwrap()) as usize;

    (
        UiItem { item: item.clone() },
        Node {
            width: Val::Px(UI_ITEM_SIZE),
            height: Val::Px(UI_ITEM_SIZE),
            ..default()
        },
        ImageNode::from_atlas_image(sheet.texture.clone(), atlas),
        Transform::from_xyz(position.x, position.y, 0.0),
        RenderLayers::layer(1),
    )
}

pub fn item_drag_started(
    event: On<ItemDragStarted>,
    mut commands: Commands,
    appearances: Res<Appearances>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    hover_state: Res<MouseHoverState>,
    stacks: Res<ItemStacks>,
    stack_item_q: Query<&Children>,
    drag_item_q: Query<Entity, With<UiItemDragging>>,
) {
    for e in drag_item_q {
        commands.entity(e).despawn();
    }

    if let ItemPlacement::Map { position, index } = &event.origin {
        let Some(stack_entity) = stacks.occupied_tiles.get(position) else {
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
    drag_item_q: Query<Entity, With<UiItemDragging>>,
) {
    let Ok(entity) = drag_item_q.single() else {
        return;
    };

    commands.entity(entity).insert(Visibility::Hidden);
}

pub fn item_move_canceled(
    _: On<ItemMoveCanceled>,
    drag_item_q: Query<(Entity, &UiItemDragging)>,
    stacks: Res<ItemStacks>,
    stack_item_q: Query<&Children>,
    mut commands: Commands,
) {
    let Ok((entity, drag_item)) = drag_item_q.single() else {
        return;
    };
    if let ItemPlacement::Map { position, index } = &drag_item.origin {
        let Some(stack_entity) = stacks.occupied_tiles.get(position) else {
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

pub fn item_move_confirmed(
    _: On<ItemMoveConfirmed>,
    drag_item_q: Query<Entity, With<UiItemDragging>>,
    mut commands: Commands,
) {
    let Ok(entity) = drag_item_q.single() else {
        return;
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
