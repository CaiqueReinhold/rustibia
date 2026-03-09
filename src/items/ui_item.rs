use bevy::{camera::visibility::RenderLayers, prelude::*};

use crate::{
    conf::ui::DRAGGED_ITEM_UI_Z,
    core::Appearances,
    items::{map_stack::ItemStacks, Item},
    main_ui::GameScaleFactor,
    map::{MouseHoverState, TilePosition},
};

pub enum ItemDragOrigin {
    Map { position: TilePosition },
    Container,
}

#[derive(Event)]
pub struct ItemDragStarted {
    pub item: Item,
    pub origin: ItemDragOrigin,
}

#[derive(Event)]
pub struct ItemDragEnded {
    pub canceled: bool,
}

#[derive(Component, Debug)]
pub struct UiItem {
    pub item: Item,
}

#[derive(Component)]
pub struct UiItemDragging {
    offset: Vec2,
    origin: ItemDragOrigin,
}

fn spawn_ui_item(
    item: &Item,
    origin: ItemDragOrigin,
    commands: &mut Commands,
    appearances: &Appearances,
    texture_atlases: &mut Assets<TextureAtlasLayout>,
    offset: &Vec2,
    position: &Vec2,
    scale: f32,
) {
    let Some(config) = appearances.sprite_configs.get(&item.config.id) else {
        return;
    };
    let Some(sheet) = appearances.sheets.get(&config.group) else {
        return;
    };
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
    commands.spawn((
        UiItem { item: item.clone() },
        UiItemDragging {
            offset: *offset,
            origin,
        },
        Node {
            width: Val::Px(32.0 * scale),
            height: Val::Px(32.0 * scale),
            ..default()
        },
        ImageNode::from_atlas_image(sheet.texture.clone(), atlas),
        Transform::from_xyz(
            position.x + offset.x,
            position.y + offset.y,
            DRAGGED_ITEM_UI_Z,
        ),
        RenderLayers::layer(1),
    ));
}

pub fn item_drag_started(
    event: On<ItemDragStarted>,
    mut commands: Commands,
    appearances: Res<Appearances>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    hover_state: Res<MouseHoverState>,
    stacks: Res<ItemStacks>,
    stack_item_q: Query<&Children>,
    scale: Res<GameScaleFactor>,
) {
    info!("Item drag started");
    let mut offset = Vec2::ZERO;
    let mut origin = ItemDragOrigin::Container;
    if let ItemDragOrigin::Map { position } = &event.origin {
        let Some(stack_entity) = stacks.occupied_tiles.get(position) else {
            return;
        };
        let Ok(stack_items) = stack_item_q.get(*stack_entity) else {
            return;
        };
        let Some(item_entity) = stack_items.last() else {
            return;
        };

        commands.entity(*item_entity).insert(Visibility::Hidden);

        offset = hover_state.world_position.unwrap() % 32.0;
        origin = ItemDragOrigin::Map {
            position: position.clone(),
        };
    }

    let offseted_position = hover_state.screen_position + offset;
    spawn_ui_item(
        &event.item,
        origin,
        &mut commands,
        &appearances,
        &mut texture_atlases,
        &offset,
        &offseted_position,
        scale.0,
    );
}

pub fn item_drag_ended(
    event: On<ItemDragEnded>,
    mut commands: Commands,
    drag_item_q: Query<(Entity, &UiItemDragging)>,
    stacks: Res<ItemStacks>,
    stack_item_q: Query<&Children>,
) {
    let Ok((entity, drag_item)) = drag_item_q.single() else {
        return;
    };

    if event.canceled {
        if let ItemDragOrigin::Map { position } = &drag_item.origin {
            let Some(stack_entity) = stacks.occupied_tiles.get(position) else {
                return;
            };
            let Ok(stack_items) = stack_item_q.get(*stack_entity) else {
                return;
            };
            for item in stack_items {
                commands.entity(*item).insert(Visibility::Visible);
            }
        }
    }

    commands.entity(entity).despawn();
}

pub fn move_dragged_item(
    ui_item_q: Query<(&mut UiTransform, &UiItemDragging)>,
    hover_state: Res<MouseHoverState>,
) {
    for (mut item_transform, dragging) in ui_item_q {
        item_transform.translation = Val2::new(
            Val::Px(hover_state.screen_position.x - dragging.offset.x),
            Val::Px(hover_state.screen_position.y + dragging.offset.y),
        );
    }
}
