use crate::{
    conf::ui::ui_colors,
    game_ui::{GameUiAssets, GameViewport},
    network::events::ShowTextMessage,
};
use bevy::prelude::*;
use bevy::{camera::visibility::RenderLayers, text::FontSmoothing};
use bevy_text_outline::TextOutline;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextMessageType {
    ActionDenied,
    Look,
}

#[derive(Component, Debug)]
pub struct TextMessage {
    timer: Timer,
    message_type: TextMessageType,
}

pub fn on_text_message(
    event: On<ShowTextMessage>,
    mut commands: Commands,
    viewport_q: Single<(&ComputedNode, &UiGlobalTransform), With<GameViewport>>,
    ui_assets: Res<GameUiAssets>,
    message_q: Query<(Entity, &TextMessage)>,
) {
    let (viewport_node, viewport_transform) = *viewport_q;
    let top = match event.message_type {
        TextMessageType::ActionDenied => {
            Val::Px(viewport_transform.translation.y + viewport_node.size().y * 0.5 - 20.0)
        }
        TextMessageType::Look => Val::Px(viewport_transform.translation.y),
    };
    let timer = match event.message_type {
        TextMessageType::ActionDenied => Timer::from_seconds(2.0, TimerMode::Once),
        TextMessageType::Look => Timer::from_seconds(5.0, TimerMode::Once),
    };
    let color = match event.message_type {
        TextMessageType::ActionDenied => Color::WHITE,
        TextMessageType::Look => ui_colors::FONT_COLOR_LOOK_MSG.into(),
    };

    for (entity, msg) in message_q {
        if msg.message_type == event.message_type {
            commands.entity(entity).despawn();
        }
    }

    commands.spawn((
        TextMessage {
            timer,
            message_type: event.message_type,
        },
        RenderLayers::layer(1),
        ZIndex(100),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(viewport_node.size().x),
            left: Val::Px(viewport_transform.translation.x - viewport_node.size().x * 0.5),
            top,
            ..default()
        },
        Text::new(event.text.clone()),
        TextFont {
            font: ui_assets.font.clone(),
            font_size: 11.0,
            ..default()
        }
        .with_font_smoothing(FontSmoothing::None),
        TextColor(color),
        TextLayout::new_with_justify(Justify::Center),
        TextOutline {
            width: 1.0,
            ..default()
        },
    ));
}

pub fn despawn_text_messages(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut TextMessage)>,
) {
    for (entity, mut text_message) in q.iter_mut() {
        text_message.timer.tick(time.delta());
        if text_message.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
