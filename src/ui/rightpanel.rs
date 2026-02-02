use bevy::prelude::*;

use crate::conf::SIDE_PANEL_WIDTH;
use crate::ui::window::{DockId, Index, UIWindowDock};

#[derive(Component)]
pub struct RightPanel;

pub fn spawn_right_panel(commands: &mut Commands, asset_server: &Res<AssetServer>) -> Entity {
    let bg_box = asset_server.load("ui/box.png");

    let slicer = TextureSlicer {
        border: BorderRect {
            min_inset: Vec2 { x: 20.0, y: 50.0 },
            max_inset: Vec2 { x: 20.0, y: 50.0 },
        },
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };

    return commands
        .spawn((
            RightPanel,
            Node {
                position_type: PositionType::Relative,
                max_width: Val::Px(SIDE_PANEL_WIDTH),
                min_width: Val::Px(SIDE_PANEL_WIDTH),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            (ImageNode {
                image: bg_box,
                ..default()
            })
            .with_mode(NodeImageMode::Sliced(slicer)),
            BoxShadow(vec![ShadowStyle {
                color: Color::BLACK.with_alpha(0.8),
                x_offset: px(0),
                y_offset: px(-2),
                spread_radius: px(1),
                blur_radius: px(5),
            }]),
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
                Index(0),
            ));
        })
        .id();
}
