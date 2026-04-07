use bevy::prelude::*;

use crate::conf::ui::{ui_colors, SIDE_PANEL_WIDTH};
use crate::game_ui::window::{DockId, Index, UIWindowDock};
use crate::game_ui::GameUiAssets;

#[derive(Component)]
pub struct LeftPanel;

pub fn spawn_left_panel(commands: &mut Commands, ui_assets: &GameUiAssets) -> Entity {
    commands
        .spawn((
            LeftPanel,
            Node {
                position_type: PositionType::Relative,
                max_width: Val::Px(SIDE_PANEL_WIDTH),
                min_width: Val::Px(SIDE_PANEL_WIDTH),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                border: UiRect {
                    top: Val::Px(0.0),
                    left: Val::Px(2.0),
                    right: Val::Px(2.0),
                    bottom: Val::Px(2.0),
                },
                ..default()
            },
            BorderColor {
                top: ui_colors::LIGHT_BORDER_COLOR.into(),
                right: ui_colors::DARK_BORDER_COLOR.into(),
                bottom: ui_colors::DARK_BORDER_COLOR.into(),
                left: ui_colors::LIGHT_BORDER_COLOR.into(),
            },
            ZIndex(1),
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    height: Val::Percent(100.0),
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                UIWindowDock { id: DockId::new() },
                Index(1),
                ImageNode {
                    image: ui_assets.background_light.clone(),
                    image_mode: NodeImageMode::Tiled {
                        tile_x: true,
                        tile_y: true,
                        stretch_value: 1.0,
                    },
                    ..default()
                },
            ));
        })
        .id()
}
