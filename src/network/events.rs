use bevy::prelude::*;

use crate::{
    actor::{Health, Mana},
    conf::map::{TILES_X, TILES_Y},
    core::{OutfitId, TextMessageType},
    items::{ContainerId, InventorySlot, ItemId},
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
    pub outfit: (OutfitId, (u8, u8, u8, u8)),
    pub speed: u16,
    pub inventory_head: Option<ItemId>,
    pub inventory_amulet: Option<ItemId>,
    pub inventory_backpack: Option<ItemId>,
    pub inventory_chest: Option<ItemId>,
    pub inventory_right_hand: Option<ItemId>,
    pub inventory_left_hand: Option<ItemId>,
    pub inventory_legs: Option<ItemId>,
    pub inventory_feet: Option<ItemId>,
    pub inventory_ring: Option<ItemId>,
    pub inventory_trinket: Option<ItemId>,
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

#[derive(Event, Debug)]
pub struct IventorySlotUpdated {
    pub slot: InventorySlot,
    pub item_id: Option<ItemId>,
}

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
            inventory_head,
            inventory_amulet,
            inventory_backpack,
            inventory_chest,
            inventory_right_hand,
            inventory_left_hand,
            inventory_legs,
            inventory_feet,
            inventory_ring,
            inventory_trinket,
        } => {
            commands.trigger(SpawnPlayer {
                position,
                _name: name,
                _level: level,
                health,
                mana,
                outfit,
                speed,
                inventory_head,
                inventory_amulet,
                inventory_backpack,
                inventory_chest,
                inventory_right_hand,
                inventory_left_hand,
                inventory_legs,
                inventory_feet,
                inventory_ring,
                inventory_trinket,
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
        ServerMessage::IventorySlotUpdated { slot, item_id } => {
            commands.trigger(IventorySlotUpdated { slot, item_id });
        }
    }
}
