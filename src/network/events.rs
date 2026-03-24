use bevy::prelude::*;

use crate::{
    actor::{Health, Mana},
    conf::map::{TILES_X, TILES_Y},
    core::OutfitId,
    map::TilePosition,
    network::{messages::ItemStack, ServerMessage},
};

#[derive(Event, Debug)]
pub struct ServerPong;

#[derive(Event, Debug)]
pub struct LoginError;

#[derive(Event, Debug)]
pub struct SpawnPlayer {
    pub position: TilePosition,
    pub _name: String,
    pub _level: u32,
    pub health: Health,
    pub mana: Mana,
    pub outfit: OutfitId,
}

#[derive(Event, Debug)]
pub struct DescribeMap {
    pub tiles: Box<[ItemStack; TILES_X * TILES_Y]>,
}

#[derive(Event, Debug)]
pub struct PlayerWalk {
    pub position: TilePosition,
    pub tiles: Box<[ItemStack]>,
}

#[derive(Event, Debug)]
pub struct PlayerPosition {
    pub position: TilePosition,
}

#[derive(Event, Debug)]
pub struct TileChanged {
    pub position: TilePosition,
    pub items: Box<ItemStack>,
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
        } => {
            commands.trigger(SpawnPlayer {
                position,
                _name: name,
                _level: level,
                health,
                mana,
                outfit,
            });
        }
        ServerMessage::DescribeMap { tiles } => {
            info!("trigger desc map");
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
        ServerMessage::MoveItemAck => {}
        ServerMessage::MoveItemDenied => {}
    }
}
