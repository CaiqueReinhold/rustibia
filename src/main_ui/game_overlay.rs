use bevy::prelude::*;

use crate::camera::GameRenderTexture;
use crate::conf::viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH};

#[derive(Component)]
pub struct GameViewport;

/// Spawn the game viewport node. The game camera renders into an offscreen
/// texture which is displayed here as an ImageNode, scaled to fill the
/// available space while preserving the game's aspect ratio.
pub fn spawn_gameviewport(commands: &mut Commands, render_texture: &GameRenderTexture) -> Entity {
    commands
        .spawn((
            GameViewport,
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .with_child((
            Node {
                // Fill available space while maintaining the game's aspect ratio.
                // CSS-style: height drives the size; width is clamped to 100%.
                height: Val::Percent(100.0),
                max_width: Val::Percent(100.0),
                aspect_ratio: Some(GAME_VIEW_WIDTH / GAME_VIEW_HEIGHT),
                ..default()
            },
            ImageNode::new(render_texture.0.clone()),
        ))
        .id()
}
