use std::collections::HashMap;

use crate::core::ChatMessageType;
use crate::game_ui::chat::state::ChannelId;

/// Which tab an inbound message belongs in.
///
/// Note the asymmetry: a channel message is keyed by `channel` and a private message
/// by `author`. A private message carries channel 0, so keying it by channel would
/// funnel every conversation into one tab.
pub fn channel_for(message_type: ChatMessageType, channel: u16, author: u16) -> ChannelId {
    match message_type {
        ChatMessageType::Local => ChannelId::Local,
        ChatMessageType::Channel => ChannelId::Server(channel),
        ChatMessageType::Private => ChannelId::Private(author),
    }
}

/// What to put on the wire for a message typed into `id`, and whether the client must
/// render its own copy.
pub struct Outbound {
    pub message_type: ChatMessageType,
    pub target: u16,
    /// True only for private messages. The server echoes local speech (it fans out
    /// with `originator: None`) and channel messages (the sender is a member), but
    /// delivers a private message solely to the recipient.
    pub echo: bool,
}

pub fn outbound_for(id: ChannelId) -> Outbound {
    match id {
        ChannelId::Local => Outbound {
            message_type: ChatMessageType::Local,
            target: 0,
            echo: false,
        },
        ChannelId::Server(channel) => Outbound {
            message_type: ChatMessageType::Channel,
            target: channel,
            echo: false,
        },
        ChannelId::Private(player) => Outbound {
            message_type: ChatMessageType::Private,
            target: player,
            echo: true,
        },
    }
}

/// Display name for a chat author. Falls back rather than failing: the server
/// introduces an author before their first message, so a miss means we lost an
/// introduction — and an unattributed line beats a dropped one.
pub fn resolve_author(names: &HashMap<u16, String>, author: u16) -> String {
    names
        .get(&author)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_speech_routes_to_the_local_tab() {
        assert_eq!(channel_for(ChatMessageType::Local, 0, 5), ChannelId::Local);
    }

    #[test]
    fn a_channel_message_routes_by_channel_not_author() {
        assert_eq!(
            channel_for(ChatMessageType::Channel, 3, 5),
            ChannelId::Server(3)
        );
    }

    /// A private conversation is keyed by the *other party*, which for an inbound
    /// message is the author — not the channel field, which is 0.
    #[test]
    fn a_private_message_routes_by_author_not_channel() {
        assert_eq!(
            channel_for(ChatMessageType::Private, 0, 7),
            ChannelId::Private(7)
        );
    }

    #[test]
    fn local_is_sent_without_echo() {
        let out = outbound_for(ChannelId::Local);
        assert!(matches!(out.message_type, ChatMessageType::Local));
        assert_eq!(out.target, 0);
        assert!(
            !out.echo,
            "the server echoes local speech back to the speaker"
        );
    }

    #[test]
    fn a_channel_is_sent_without_echo() {
        let out = outbound_for(ChannelId::Server(4));
        assert!(matches!(out.message_type, ChatMessageType::Channel));
        assert_eq!(out.target, 4);
        assert!(!out.echo, "the sender is a member, so the server echoes");
    }

    /// The one case the server does not echo: it delivers a private message only to
    /// the recipient, so the sender must render its own copy.
    #[test]
    fn a_private_message_is_echoed_locally() {
        let out = outbound_for(ChannelId::Private(9));
        assert!(matches!(out.message_type, ChatMessageType::Private));
        assert_eq!(out.target, 9);
        assert!(
            out.echo,
            "the server delivers private messages only to the recipient"
        );
    }

    #[test]
    fn a_known_author_resolves_to_their_name() {
        let mut names = HashMap::new();
        names.insert(4u16, "Rizael".to_owned());
        assert_eq!(resolve_author(&names, 4), "Rizael");
    }

    /// The server introduces an author before their first message, so this should not
    /// happen — but dropping the message would be worse than an unattributed line.
    #[test]
    fn an_unknown_author_falls_back_rather_than_dropping() {
        assert_eq!(resolve_author(&HashMap::new(), 4), "Unknown");
    }
}
