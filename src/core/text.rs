use crate::network::events::TextMessage;
use bevy::prelude::*;

#[derive(Debug, Clone)]
pub enum TextMessageType {
    ActionDenied,
}

pub fn on_text_message(event: On<TextMessage>) {
    info!("Text message [{:?}] {}", event.message_type, event.text);
}
