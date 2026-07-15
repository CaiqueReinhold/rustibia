use bevy::prelude::*;

use crate::conf::ui::z_index::Z_MAIN_UI;
use crate::conf::ui::{SIDE_PANEL_WIDTH, ui_colors};
use crate::game_ui::GameUiAssets;
use crate::game_ui::window::{DockId, Index, UIWindowDock};

#[derive(Component)]
pub struct RightPanel;

#[derive(Component)]
pub struct RightPanelDock;

pub fn spawn_right_panel(commands: &mut Commands, ui_assets: &GameUiAssets) -> Entity {
    commands
        .spawn((
            RightPanel,
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
            ZIndex(Z_MAIN_UI),
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
                Index(0),
                RightPanelDock,
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
