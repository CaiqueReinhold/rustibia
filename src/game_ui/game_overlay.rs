use bevy::prelude::*;

use crate::camera::GameRenderTexture;
use crate::conf::ui::ui_colors;
use crate::conf::viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH};
use crate::game_ui::GameUiAssets;

#[derive(Component)]
pub struct GameViewport;

#[derive(Component)]
pub(super) struct GameViewportContainer;

#[derive(Component)]
pub(super) struct GameViewportBorder;

const ASPECT_RATIO: f32 = GAME_VIEW_WIDTH / GAME_VIEW_HEIGHT;
// border(1) + padding(1) + margin(2) = 4px per side, 8px per axis
const BORDER_OVERHEAD: f32 = 8.0;

pub(super) fn update_viewport_size(
    container_query: Query<&ComputedNode, (With<GameViewportContainer>, Changed<ComputedNode>)>,
    mut border_query: Query<&mut Node, With<GameViewportBorder>>,
) {
    let Ok(computed) = container_query.single() else {
        return;
    };
    let Ok(mut node) = border_query.single_mut() else {
        return;
    };

    let avail_w = computed.size().x - BORDER_OVERHEAD;
    let avail_h = computed.size().y - BORDER_OVERHEAD;

    let (w, h) = if avail_w / avail_h > ASPECT_RATIO {
        // height-limited: fit to height
        let h = avail_h;
        (h * ASPECT_RATIO, h)
    } else {
        // width-limited: fit to width
        let w = avail_w;
        (w, w / ASPECT_RATIO)
    };

    node.width = Val::Px(w);
    node.height = Val::Px(h);
}

/// Spawn the game viewport node. The game camera renders into an offscreen
/// texture which is displayed here as an ImageNode, scaled to fill the
/// available space while preserving the game's aspect ratio.
pub fn spawn_gameviewport(
    commands: &mut Commands,
    render_texture: &GameRenderTexture,
    ui_assets: &GameUiAssets,
) -> Entity {
    commands
        .spawn((
            GameViewportContainer,
            Node {
                max_width: Val::Percent(100.0),
                max_height: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            ImageNode {
                image: ui_assets.background_light.clone(),
                image_mode: NodeImageMode::Tiled {
                    tile_x: true,
                    tile_y: true,
                    stretch_value: 1.0,
                },
                ..default()
            },
        ))
        .with_children(|view| {
            view.spawn((
                GameViewportBorder,
                Node {
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(1.0)),
                    margin: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor {
                    top: ui_colors::DARK_BORDER_COLOR.into(),
                    right: ui_colors::LIGHT_BORDER_COLOR.into(),
                    bottom: ui_colors::LIGHT_BORDER_COLOR.into(),
                    left: ui_colors::DARK_BORDER_COLOR.into(),
                },
            ))
            .with_child((
                GameViewport,
                Node {
                    height: Val::Percent(100.0),
                    width: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(render_texture.0.clone()),
            ));
        })
        .id()
}
