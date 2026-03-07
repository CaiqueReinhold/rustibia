use bevy::prelude::*;

use crate::conf::ui::CHAT_BOX_HEIGHT;

pub fn spawn_chat(commands: &mut Commands, asset_server: &Res<AssetServer>) -> Entity {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let bg_box = asset_server.load("ui/box_chat.png");
    let slicer = TextureSlicer {
        border: BorderRect {
            min_inset: Vec2 { x: 50.0, y: 50.0 },
            max_inset: Vec2 { x: 50.0, y: 50.0 },
        },
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Relative,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                min_height: Val::Px(CHAT_BOX_HEIGHT - 2.0),
                max_height: Val::Px(CHAT_BOX_HEIGHT - 2.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect {
                    top: Val::Px(2.0),
                    ..default()
                },
                ..default()
            },
            (ImageNode {
                image: bg_box,
                ..default()
            })
            .with_mode(NodeImageMode::Sliced(slicer)),
            BoxShadow(vec![ShadowStyle {
                color: Color::BLACK.with_alpha(0.8),
                x_offset: px(-2),
                y_offset: px(0),
                spread_radius: px(1),
                blur_radius: px(5),
            }]),
            BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.0)),
            Name::new("Chat"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Chat Window"),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Visibility::Visible,
                InheritedVisibility::default(),
            ));
        })
        .id()
}
