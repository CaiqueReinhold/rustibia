use bevy::prelude::*;

use crate::conf::ui::chat as conf;
use crate::game_ui::chat::events::{AppendChatMessage, CloseChannel, OpenChannel};
use crate::game_ui::chat::routing::{channel_for, resolve_author};
use crate::game_ui::chat::state::{ChannelConfig, ChannelId, ChatMessage, ChatState};
use crate::network::events::{ChannelListReceived, ChatMessageReceived, PlayerIntroduced};
use crate::network::{ClientMessage, SendMessage};

/// Server channels are static config, so the list is fetched once and cached.
pub fn request_channels(mut commands: Commands) {
    commands.trigger(SendMessage(ClientMessage::RequestChannels));
}

pub fn on_channel_list_received(event: On<ChannelListReceived>, mut state: ResMut<ChatState>) {
    state.available = event
        .channels
        .iter()
        .map(|(id, name)| ChannelConfig {
            id: ChannelId::Server(*id),
            name: name.clone(),
            closeable: true,
            text_color: Color::Srgba(conf::LOCAL_CHANNEL_COLOR),
        })
        .collect();
}

/// Records the name only. It must NOT open a tab: the server also sends this ahead of
/// local and channel messages from unknown authors, so opening here would spawn a
/// private tab for anyone who speaks near you.
pub fn on_player_introduced(
    event: On<PlayerIntroduced>,
    mut state: ResMut<ChatState>,
    mut commands: Commands,
) {
    state
        .player_names
        .insert(event.local_id, event.name.clone());

    if state.pending_pm_open.as_deref() == Some(event.name.as_str()) {
        state.pending_pm_open = None;
        commands.trigger(OpenChannel {
            config: ChannelConfig {
                id: ChannelId::Private(event.local_id),
                name: event.name.clone(),
                closeable: true,
                text_color: Color::Srgba(conf::LOCAL_CHANNEL_COLOR),
            },
        });
    }
}

pub fn on_chat_message_received(
    event: On<ChatMessageReceived>,
    state: Res<ChatState>,
    mut commands: Commands,
) {
    let channel_id = channel_for(event.message_type, event.channel, event.author);

    let author = resolve_author(&state.player_names, event.author);

    // A private message from someone we have no tab for opens one. The name is known
    // because the server always introduces an author before their first message.
    if matches!(channel_id, ChannelId::Private(_)) && !state.is_open(channel_id) {
        commands.trigger(OpenChannel {
            config: ChannelConfig {
                id: channel_id,
                name: author.clone(),
                closeable: true,
                text_color: Color::Srgba(conf::LOCAL_CHANNEL_COLOR),
            },
        });
    } else if !state.is_open(channel_id) {
        // A channel message for a tab we already closed — a race against
        // CLI_CLOSE_CHANNEL. Drop it.
        return;
    }

    commands.trigger(AppendChatMessage {
        message: ChatMessage {
            text: event.text.clone(),
            channel_id: Some(channel_id),
            author: Some(author),
        },
    });
}

/// Wire sends live on the state events rather than the click sites: there is no case
/// where a `Server` channel is opened or closed locally without telling the server, and
/// `Local`/`Private` simply do not send.
pub fn on_open_channel_wire(event: On<OpenChannel>, mut commands: Commands) {
    if let ChannelId::Server(id) = event.config.id {
        commands.trigger(SendMessage(ClientMessage::OpenChannel { channel: id }));
    }
}

pub fn on_close_channel_wire(event: On<CloseChannel>, mut commands: Commands) {
    if let ChannelId::Server(id) = event.channel_id {
        commands.trigger(SendMessage(ClientMessage::CloseChannel { channel: id }));
    }
}
