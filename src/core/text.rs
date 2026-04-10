use crate::{
    game_ui::{GameUiAssets, GameViewport},
    network::events::ShowTextMessage,
};
use bevy::prelude::*;
use bevy::{camera::visibility::RenderLayers, text::FontSmoothing};
use bevy_text_outline::TextOutline;

#[derive(Debug, Clone)]
pub enum TextMessageType {
    ActionDenied,
}

#[derive(Component, Debug)]
pub struct TextMessage {
    timer: Timer,
}

pub fn on_text_message(
    event: On<ShowTextMessage>,
    mut commands: Commands,
    viewport_q: Single<(&ComputedNode, &UiGlobalTransform), With<GameViewport>>,
    ui_assets: Res<GameUiAssets>,
) {
    let (viewport_node, viewport_transform) = *viewport_q;

    commands.spawn((
        TextMessage {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
        },
        RenderLayers::layer(1),
        ZIndex(100),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(viewport_node.size().x),
            left: Val::Px(viewport_transform.translation.x - viewport_node.size().x * 0.5),
            top: Val::Px(viewport_transform.translation.y + viewport_node.size().y * 0.5 - 20.0),
            ..default()
        },
        Text::new(event.text.clone()),
        TextFont {
            font: ui_assets.font.clone(),
            font_size: 11.0,
            ..default()
        }
        .with_font_smoothing(FontSmoothing::None),
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
