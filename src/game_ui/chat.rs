use bevy::prelude::*;

use crate::{
    conf::ui::{ui_colors, CHAT_BOX_HEIGHT},
    game_ui::GameUiAssets,
};

pub fn spawn_chat(commands: &mut Commands, ui_assets: &GameUiAssets) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Relative,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                min_height: Val::Px(CHAT_BOX_HEIGHT),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor {
                top: ui_colors::LIGHT_BORDER_COLOR.into(),
                right: ui_colors::DARK_BORDER_COLOR.into(),
                bottom: ui_colors::DARK_BORDER_COLOR.into(),
                left: ui_colors::LIGHT_BORDER_COLOR.into(),
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
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
                .with_children(|inner| {
                    inner.spawn((
                        Text::new("Chat Window"),
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Visibility::Visible,
                        InheritedVisibility::default(),
                    ));
                });
        })
        .id()
}
