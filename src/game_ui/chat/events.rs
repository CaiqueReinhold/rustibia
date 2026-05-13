use bevy::ecs::event::Event;

use crate::game_ui::chat::state::{ChannelConfig, ChannelId, ChatMessage};

#[derive(Event)]
pub struct OpenChannel {
    pub config: ChannelConfig,
}

#[derive(Event)]
pub struct CloseChannel {
    pub channel_id: ChannelId,
}

#[derive(Event)]
pub struct ActivateChannel {
    pub channel_id: ChannelId,
}

#[derive(Event)]
pub struct AppendChatMessage {
    pub message: ChatMessage,
}

#[derive(Event)]
pub struct EnterChatMode;

#[derive(Event)]
pub struct ExitChatMode;

#[derive(Event)]
pub struct SubmitChatInput {
    pub text: String,
}

/// Fired by the state-mutation observer after `AppendChatMessage` has been
/// stored. UI systems read post-mutation `ChatState` from this.
#[derive(Event)]
pub struct MessageAppendedUi {
    pub channel_id: ChannelId,
    pub sequence: u64,
}

/// Fired when a stored message is dropped because the channel exceeded
/// `history_cap`. UI systems despawn the matching node.
#[derive(Event)]
pub struct MessageTrimmedUi {
    pub channel_id: ChannelId,
    pub sequence: u64,
}
