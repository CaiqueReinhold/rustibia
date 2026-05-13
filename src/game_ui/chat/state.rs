use std::collections::VecDeque;

use bevy::color::Color;
use bevy::ecs::resource::Resource;
use bevy::prelude::*;
use chrono::{DateTime, Local};

use crate::conf::ui::chat as conf;
use crate::game_ui::chat::events::{
    ActivateChannel, AppendChatMessage, CloseChannel, EnterChatMode, ExitChatMode,
    MessageAppendedUi, MessageTrimmedUi, OpenChannel,
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ChannelId(pub u32);

pub const LOCAL_CHANNEL_ID: ChannelId = ChannelId(0);

#[derive(Clone, Debug)]
pub struct ChannelConfig {
    pub id: ChannelId,
    pub name: &'static str,
    pub closeable: bool,
    pub text_color: Color,
}

#[derive(Debug)]
pub struct ChatMessage {
    pub text: String,
    pub channel_id: Option<ChannelId>,
}

#[derive(Debug)]
pub struct StoredMessage {
    pub timestamp: DateTime<Local>,
    pub sequence: u64,
    pub message: ChatMessage,
}

#[derive(Debug)]
pub struct Channel {
    pub config: ChannelConfig,
    pub messages: VecDeque<StoredMessage>,
    pub unread: bool,
    pub scroll_pinned_bottom: bool,
    pub next_sequence: u64,
}

impl Channel {
    pub fn new(config: ChannelConfig) -> Self {
        Self {
            config,
            messages: VecDeque::new(),
            unread: false,
            scroll_pinned_bottom: true,
            next_sequence: 0,
        }
    }
}

#[derive(Resource, Debug)]
pub struct ChatState {
    pub channels: Vec<Channel>,
    pub active: ChannelId,
    pub history_cap: usize,
    pub available: Vec<ChannelConfig>,
}

impl Default for ChatState {
    fn default() -> Self {
        let local = local_channel_config();
        let world = ChannelConfig {
            id: ChannelId(2),
            name: "World Chat",
            closeable: true,
            text_color: Color::Srgba(conf::LOCAL_CHANNEL_COLOR),
        };
        Self {
            channels: vec![Channel::new(local.clone()), Channel::new(world)],
            active: LOCAL_CHANNEL_ID,
            history_cap: conf::HISTORY_CAP_DEFAULT,
            available: Vec::new(),
        }
    }
}

impl ChatState {
    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.iter().find(|c| c.config.id == id)
    }

    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|c| c.config.id == id)
    }

    pub fn is_open(&self, id: ChannelId) -> bool {
        self.channels.iter().any(|c| c.config.id == id)
    }
}

#[derive(Resource, Default, Debug)]
pub struct ChatMode {
    pub active: bool,
}

pub fn local_channel_config() -> ChannelConfig {
    ChannelConfig {
        id: LOCAL_CHANNEL_ID,
        name: conf::LOCAL_CHANNEL_NAME,
        closeable: false,
        text_color: Color::Srgba(conf::LOCAL_CHANNEL_COLOR),
    }
}

pub fn on_open_channel(
    event: On<OpenChannel>,
    mut state: ResMut<ChatState>,
    mut commands: Commands,
) {
    let id = event.config.id;
    if state.is_open(id) {
        commands.trigger(ActivateChannel { channel_id: id });
        return;
    }
    state.channels.push(Channel::new(event.config.clone()));
    commands.trigger(ActivateChannel { channel_id: id });
}

pub fn on_close_channel(
    event: On<CloseChannel>,
    mut state: ResMut<ChatState>,
    mut commands: Commands,
) {
    let Some(channel) = state.channel(event.channel_id) else {
        return;
    };
    if !channel.config.closeable {
        return;
    }
    let was_active = state.active == event.channel_id;
    state.channels.retain(|c| c.config.id != event.channel_id);
    if was_active {
        commands.trigger(ActivateChannel {
            channel_id: LOCAL_CHANNEL_ID,
        });
    }
}

pub fn on_activate_channel(event: On<ActivateChannel>, mut state: ResMut<ChatState>) {
    if !state.is_open(event.channel_id) {
        return;
    }
    state.active = event.channel_id;
    if let Some(c) = state.channel_mut(event.channel_id) {
        c.unread = false;
        c.scroll_pinned_bottom = true;
    }
}

pub fn on_append_chat_message(
    event: On<AppendChatMessage>,
    mut state: ResMut<ChatState>,
    mut commands: Commands,
) {
    let cap = state.history_cap;
    let active = state.active;
    let target_ids: Vec<ChannelId> = match event.message.channel_id {
        Some(id) => vec![id],
        None => state.channels.iter().map(|c| c.config.id).collect(),
    };

    for id in target_ids {
        let Some(channel) = state.channel_mut(id) else {
            continue;
        };
        let sequence = channel.next_sequence;
        channel.next_sequence += 1;

        let stored = StoredMessage {
            timestamp: Local::now(),
            sequence,
            message: ChatMessage {
                text: event.message.text.clone(),
                channel_id: event.message.channel_id,
            },
        };

        channel.messages.push_back(stored);

        let mut trimmed: Option<u64> = None;
        if channel.messages.len() > cap
            && let Some(popped) = channel.messages.pop_front()
        {
            trimmed = Some(popped.sequence);
        }

        if id != active {
            channel.unread = true;
        }

        if let Some(seq) = trimmed {
            commands.trigger(MessageTrimmedUi {
                channel_id: id,
                sequence: seq,
            });
        }
        commands.trigger(MessageAppendedUi {
            channel_id: id,
            sequence,
        });
    }
}

pub fn on_enter_chat_mode(_event: On<EnterChatMode>, mut mode: ResMut<ChatMode>) {
    mode.active = true;
}

pub fn on_exit_chat_mode(_event: On<ExitChatMode>, mut mode: ResMut<ChatMode>) {
    mode.active = false;
}
