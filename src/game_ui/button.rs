use bevy::prelude::*;

use crate::conf::ui::{dialog as conf, ui_colors};
use crate::game_ui::GameUiAssets;

/// The Tibia-style raised button, used by modal dialogs and by the right-panel
/// button row. `panel_button_hover` styles every one of them.
#[derive(Component)]
pub struct PanelButton;

pub fn spawn_panel_button(
    commands: &mut Commands,
    label: impl Into<String>,
    ui_assets: &GameUiAssets,
) -> Entity {
    commands
        .spawn((
            PanelButton,
            Button,
            Node {
                min_width: Val::Px(conf::BUTTON_MIN_WIDTH),
                height: Val::Px(conf::BUTTON_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor {
                top: ui_colors::LIGHT_BORDER_COLOR.into(),
                right: ui_colors::DARK_BORDER_COLOR.into(),
                bottom: ui_colors::DARK_BORDER_COLOR.into(),
                left: ui_colors::LIGHT_BORDER_COLOR.into(),
            },
            BackgroundColor(conf::BUTTON_COLOR.into()),
        ))
        .with_child((
            Text::new(label.into()),
            TextFont {
                font: ui_assets.font.clone(),
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id()
}

pub fn panel_button_hover(
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<PanelButton>),
    >,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::None => BackgroundColor(conf::BUTTON_COLOR.into()),
            _ => BackgroundColor(conf::BUTTON_HOVER_COLOR.into()),
        };
    }
}
