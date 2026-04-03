use bevy::prelude::*;

use crate::{
    actor::{Health, Mana},
    conf::map::{TILES_X, TILES_Y},
    core::{OutfitId, TextMessageType},
    items::{ContainerId, ItemId},
    map::Position,
    network::{messages::ItemStack, ServerMessage},
};

#[derive(Event, Debug)]
pub struct ServerPong;

#[derive(Event, Debug)]
pub struct LoginError;

#[derive(Event, Debug)]
pub struct SpawnPlayer {
    pub position: Position,
    pub _name: String,
    pub _level: u16,
    pub health: Health,
    pub mana: Mana,
    pub outfit: OutfitId,
    pub speed: u16,
}

#[derive(Event, Debug)]
pub struct DescribeMap {
    pub tiles: Box<[ItemStack; TILES_X * TILES_Y]>,
}

#[derive(Event, Debug)]
pub struct PlayerWalk {
    pub position: Position,
    pub tiles: Box<[ItemStack]>,
}

#[derive(Event, Debug)]
pub struct PlayerPosition {
    pub position: Position,
}

#[derive(Event, Debug)]
pub struct TileChanged {
    pub position: Position,
    pub items: Box<ItemStack>,
}

#[derive(Event, Debug)]
pub struct MoveItemResult {
    pub success: bool,
}

#[derive(Event, Debug)]
pub struct TextMessage {
    pub text: String,
    pub message_type: TextMessageType,
}

#[derive(Event, Debug)]
pub struct UseItemAck;

#[derive(Event, Debug)]
pub struct OpenContainer {
    pub container_id: ContainerId,
    pub capacity: u8,
    pub has_parent: bool,
    pub title: String,
    pub items: Box<[Option<(ItemId, u8)>]>,
}

#[derive(Event, Debug)]
pub struct UpdateContainer {
    pub container_id: ContainerId,
    pub items: Box<[Option<(ItemId, u8)>]>,
}

#[derive(Event, Debug)]
pub struct ContainerClosed {
    pub container_id: ContainerId,
}

#[derive(Event, Debug)]
pub struct PlayerWalkDenied;

pub fn route_event(msg: ServerMessage, commands: &mut Commands) {
    match msg {
        ServerMessage::Pong => {
            commands.trigger(ServerPong);
        }
        ServerMessage::LoginError => commands.trigger(LoginError),
        ServerMessage::DescribePlayer {
            position,
            name,
            level,
            health,
            mana,
            outfit,
            speed,
        } => {
            commands.trigger(SpawnPlayer {
                position,
                _name: name,
                _level: level,
                health,
                mana,
                outfit,
                speed,
            });
        }
        ServerMessage::DescribeMap { tiles } => {
            commands.trigger(DescribeMap { tiles });
        }
        ServerMessage::PlayerWalkAck { position, tiles } => {
            commands.trigger(PlayerWalk { position, tiles });
        }
        ServerMessage::TileChanged { position, items } => {
            commands.trigger(TileChanged { position, items });
        }
        ServerMessage::PlayerPosition { position } => {
            commands.trigger(PlayerPosition { position });
        }
        ServerMessage::MoveItemAck => {
            commands.trigger(MoveItemResult { success: true });
        }
        ServerMessage::MoveItemDenied => {
            commands.trigger(MoveItemResult { success: false });
        }
        ServerMessage::TextMessage { text, message_type } => {
            commands.trigger(TextMessage { text, message_type });
        }
        ServerMessage::UseItemAck => {
            commands.trigger(UseItemAck);
        }
        ServerMessage::OpenContainer {
            container_id,
            capacity,
            has_parent,
            title,
            items,
        } => {
            commands.trigger(OpenContainer {
                container_id,
                capacity,
                has_parent,
                title,
                items,
            });
        }
        ServerMessage::UpdateContainer {
            container_id,
            items,
        } => {
            commands.trigger(UpdateContainer {
                container_id,
                items,
            });
        }
        ServerMessage::ContainerClosed { container_id } => {
            commands.trigger(ContainerClosed { container_id });
        }
        ServerMessage::PlayerWalkDenied => {
            commands.trigger(PlayerWalkDenied);
        }
    }
}
