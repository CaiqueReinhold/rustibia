use std::sync::Arc;

use bevy::{camera::visibility::RenderLayers, prelude::*};

use crate::{
    conf::ui::{z_index::DRAGGED_ITEM_UI_Z, UI_ITEM_SIZE},
    core::{Appearances, SpriteAnimation},
    items::{
        instancing::ItemState, Item, ItemDragEnded, ItemDragStarted, ItemMoveCanceled,
        ItemMoveConfirmed, ItemPlacement,
    },
    player::MouseHoverState,
};

#[derive(Component, Debug)]
#[allow(dead_code)]
pub struct UiItem {
    pub item: Arc<Item>,
}

/// Drives frame animation for UI items (inventory, containers, dragged item).
/// Mirrors the GPU shader logic in items.wgsl on the CPU.
#[derive(Component, Debug)]
pub struct UiItemAnimation {
    /// Number of animation phases. 1 means static.
    pub phase_count: u32,
    /// Seconds per phase. 0.0 means static (no animation).
    pub phase_duration: f32,
    /// Atlas sprite IDs indexed as `sprite_ids[phase]`.
    pub sprite_ids: Vec<u32>,
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

    let (phase_count, phase_duration) = match &config.animation {
        SpriteAnimation::Static => (1, 0.0),
        SpriteAnimation::Uniform {
            phase_count,
            phase_duration,
        } => (*phase_count, phase_duration.as_secs_f32()),
        // NonUniform has variable per-phase durations; treat as static (same as GPU shader).
        SpriteAnimation::NonUniform { .. } => (1, 0.0),
    };

    (
        UiItem { item: item.clone() },
        UiItemAnimation {
            phase_count,
            phase_duration,
            sprite_ids: config.sprite_ids.clone(),
        },
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

/// Advances the `TextureAtlas` index for animated UI items each frame.
///
/// Mirrors the GPU `get_animation_phase` logic from `items.wgsl`:
/// `phase = floor(time / phase_duration) % phase_count`
/// then `atlas_index = sprite_ids[phase]`.
pub fn animate_ui_items(time: Res<Time>, mut query: Query<(&UiItemAnimation, &mut ImageNode)>) {
    let elapsed = time.elapsed_secs();
    for (anim, mut image_node) in &mut query {
        if anim.phase_duration == 0.0 || anim.phase_count <= 1 {
            continue;
        }
        let phase = ((elapsed / anim.phase_duration).floor() as u32) % anim.phase_count;
        if let Some(atlas) = &mut image_node.texture_atlas {
            atlas.index = anim.sprite_ids[phase as usize] as usize;
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
    state: Res<ItemState>,
    stack_item_q: Query<&Children>,
    mut commands: Commands,
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
