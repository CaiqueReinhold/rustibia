use std::fmt::Display;

use asynchronous_codec::{Decoder, Encoder};
use bytes::{Buf, BufMut, BytesMut};
use thiserror::Error;

use crate::{
    agent::{AgentId, FacingDirection, Health, Mana, WalkingDirection},
    conf::map::{STACK_MAX_VISIBLE_ITEMS, TILES_X, TILES_Y},
    core::{ChatMessageType, OutfitColors, OutfitId, TextMessageType},
    items::{ContainerId, InventorySlot, ItemId},
    map::Position,
};

pub type ItemStack = [Option<(ItemId, u8)>; STACK_MAX_VISIBLE_ITEMS];

// client
const CLI_PING: u8 = 0;
const CLI_LOGIN: u8 = 1;
const CLI_MOVE_PLAYER: u8 = 2;
const CLI_GET_PLAYER_POS: u8 = 3;
const CLI_MOVE_ITEM: u8 = 4;
const CLI_USE_ITEM: u8 = 5;
const CLI_CLOSE_CONTAINER: u8 = 6;
const CLI_OPEN_PARENT_CONTAINER: u8 = 7;
const CLI_CHANGE_DIRECTION: u8 = 8;
const CLI_LOGOUT: u8 = 9;
const CLI_USE_ITEM_WITH: u8 = 10;
const CLI_LOOK: u8 = 11;
const CLI_SAY: u8 = 12;
const CLI_REQUEST_CHANNELS: u8 = 13;
const CLI_OPEN_CHANNEL: u8 = 14;
const CLI_CLOSE_CHANNEL: u8 = 15;
const CLI_OPEN_PM_CHAT: u8 = 16;

#[derive(Clone, Debug)]
pub enum ClientMessage {
    Ping,
    Login {
        auth_token: String,
    },
    MovePlayer {
        direction: WalkingDirection,
    },
    GetPlayerPosition,
    MoveItem {
        from: Position,
        item_id: ItemId,
        amount: u8,
        stack_index: u8,
        to: Position,
    },
    UseItem {
        position: Position,
        item_id: ItemId,
        stack_index: u8,
    },
    CloseContainer {
        container_id: ContainerId,
    },
    OpenParentContainer {
        container_id: ContainerId,
    },
    ChangeDirection {
        direction: FacingDirection,
    },
    Logout,
    UseItemWith {
        source: Position,
        source_item_id: ItemId,
        source_index: u8,
        target: Position,
        target_item_id: ItemId,
        target_index: u8,
    },
    Look {
        position: Position,
    },
    Say {
        message: String,
        message_type: ChatMessageType,
        target: u16,
    },
    RequestChannels,
    OpenChannel {
        channel: u16,
    },
    CloseChannel {
        channel: u16,
    },
    OpenPmChat {
        name: String,
    },
}

// server
const MSG_PONG: u8 = 0;
const MSG_LOGIN_ERROR: u8 = 1;
const MSG_DESCRIBE_MAP: u8 = 2;
const MSG_TILE_CHANGED: u8 = 3;
const MSG_PLAYER_WALK_ACK: u8 = 4;
const MSG_PLAYER_POS: u8 = 5;
const MSG_DESCRIBE_PLAYER: u8 = 6;
const MSG_TEXT_MESSAGE: u8 = 7;
const MSG_OPEN_CONTAINER: u8 = 8;
const MSG_UPDATE_CONTAINER: u8 = 9;
const MSG_CONTAINER_CLOSED: u8 = 10;
const MSG_PLAYER_WALK_DENIED: u8 = 11;
const MSG_INVETORY_SLOT_UPDATED: u8 = 12;
const MSG_PLAYER_CAPACITY_UPDATED: u8 = 13;
const MSG_AGENT_DIRECTION_CHANGED: u8 = 14;
const MSG_REMOVE_AGENT: u8 = 15;
const MSG_MOVE_AGENT: u8 = 16;
const MSG_SPAWN_AGENT: u8 = 17;
const MSG_TELEPORT_AGENT: u8 = 18;
const MSG_CHAT_MESSAGE: u8 = 19;
const MSG_CHANNEL_LIST: u8 = 20;
const MSG_INTRODUCE_PLAYER: u8 = 21;

#[derive(Clone, Debug)]
pub enum ServerMessage {
    Pong,
    LoginError,
    DescribePlayer {
        agent_id: AgentId,
        position: Position,
        facing: FacingDirection,
        name: String,
        level: u16,
        health: Health,
        mana: Mana,
        outfit: (OutfitId, (u8, u8, u8, u8)),
        speed: u16,
        capacity: u32,
        inventory_head: Option<ItemId>,
        inventory_amulet: Option<ItemId>,
        inventory_backpack: Option<ItemId>,
        inventory_chest: Option<ItemId>,
        inventory_right_hand: Option<ItemId>,
        inventory_left_hand: Option<ItemId>,
        inventory_legs: Option<ItemId>,
        inventory_feet: Option<ItemId>,
        inventory_ring: Option<ItemId>,
        inventory_trinket: Option<ItemId>,
    },
    DescribeMap {
        tiles: Box<[ItemStack; TILES_X * TILES_Y]>,
        floor: u8,
        center: Position,
    },
    TileChanged {
        position: Position,
        items: Box<ItemStack>,
    },
    PlayerWalkAck {
        position: Position,
        tiles: Vec<(u8, Box<[ItemStack]>)>,
    },
    PlayerPosition {
        position: Position,
    },
    TextMessage {
        text: String,
        message_type: TextMessageType,
    },
    OpenContainer {
        container_id: ContainerId,
        capacity: u8,
        has_parent: bool,
        title: String,
        items: Box<[Option<(ItemId, u8)>]>,
    },
    UpdateContainer {
        container_id: ContainerId,
        items: Box<[Option<(ItemId, u8)>]>,
    },
    ContainerClosed {
        container_id: ContainerId,
    },
    PlayerWalkDenied,
    IventorySlotUpdated {
        slot: InventorySlot,
        item_id: Option<ItemId>,
    },
    PlayerCapacityUpdated {
        capacity: u32,
    },
    AgentChangedDirection {
        agent_id: AgentId,
        facing: FacingDirection,
    },
    RemoveAgent {
        agent_id: AgentId,
    },
    MoveAgent {
        agent_id: AgentId,
        direction: WalkingDirection,
        from: Position,
    },
    SpawnAgent {
        agent_id: AgentId,
        outfit: (OutfitId, OutfitColors),
        position: Position,
        facing: FacingDirection,
        name: String,
        health: Health,
        speed: u16,
    },
    TeleportAgent {
        agent_id: AgentId,
        position: Position,
    },
    ChatMessage {
        author: u16,
        message_type: ChatMessageType,
        channel: u16,
        text: String,
    },
    ChannelList {
        channels: Vec<(u16, String)>,
    },
    IntroducePlayer {
        local_id: u16,
        name: String,
    },
}

impl Display for ServerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerMessage::DescribeMap { center, floor, .. } => {
                write!(f, "DescribeMap {{ center: {}, floor: {} }}", center, floor)
            }
            ServerMessage::PlayerWalkAck { position, tiles } => {
                write!(
                    f,
                    "PlayerWalkAck {{ position: {}, floors: {:?} }}",
                    position,
                    tiles.iter().map(|i| i.0).collect::<Vec<u8>>()
                )
            }
            ServerMessage::TileChanged { position, .. } => {
                write!(f, "TileChanged {{ position: {} }}", position)
            }
            ServerMessage::OpenContainer { container_id, .. } => {
                write!(f, "OpenContainer {{ container_id: {} }}", container_id)
            }
            ServerMessage::UpdateContainer { container_id, .. } => {
                write!(f, "UpdateContainer {{ container_id: {} }}", container_id)
            }
            msg => {
                write!(f, "{:?}", msg)
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum MessageDecodeError {
    #[error("Read error")]
    ReadError(#[from] std::io::Error),
    #[error("Wrong sequence")]
    WrongSequence,
    #[error("Payload ended mid-field")]
    Truncated,
    #[error("{0} unread byte(s) left in the payload")]
    TrailingBytes(usize),
}

/// A bounds-checked cursor over a single frame's payload.
///
/// Every read returns [`MessageDecodeError::Truncated`] instead of panicking
/// when the payload runs out, which is what keeps a malformed or truncated
/// packet from killing the connection task.
struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }

    fn remaining(&self) -> usize {
        self.buf.len()
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], MessageDecodeError> {
        if self.buf.len() < len {
            return Err(MessageDecodeError::Truncated);
        }
        let (head, tail) = self.buf.split_at(len);
        self.buf = tail;
        Ok(head)
    }

    fn read_u8(&mut self) -> Result<u8, MessageDecodeError> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_u16_le(&mut self) -> Result<u16, MessageDecodeError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn read_u32_le(&mut self) -> Result<u32, MessageDecodeError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Reads a `len`-byte string. Invalid UTF-8 is replaced rather than
    /// rejected — a mangled name is not worth dropping the connection over.
    fn read_string(&mut self, len: usize) -> Result<String, MessageDecodeError> {
        Ok(String::from_utf8_lossy(self.read_bytes(len)?).into_owned())
    }
}

pub struct GameMessageCodec {}

impl Decoder for GameMessageCodec {
    type Item = ServerMessage;
    type Error = MessageDecodeError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.len() < 2 {
            return Ok(None);
        }

        let payload_len = u16::from_le_bytes([buf[0], buf[1]]) as usize;

        if buf.len() < 2 + payload_len {
            return Ok(None);
        }

        // Split the frame off before decoding anything: every read below is
        // then bounded by this payload, so a corrupt message can neither run
        // into the next frame's bytes nor leave the stream misaligned. The
        // frame is consumed either way, so an error here is reportable
        // without desyncing the framing.
        buf.advance(2);
        let payload = buf.split_to(payload_len);

        let mut reader = Reader::new(&payload);
        let message = decode_message(&mut reader)?;
        if reader.remaining() != 0 {
            return Err(MessageDecodeError::TrailingBytes(reader.remaining()));
        }

        Ok(Some(message))
    }
}

fn decode_message(buf: &mut Reader) -> Result<ServerMessage, MessageDecodeError> {
    match buf.read_u8()? {
        MSG_PONG => Ok(ServerMessage::Pong),
        MSG_LOGIN_ERROR => Ok(ServerMessage::LoginError),
        MSG_DESCRIBE_PLAYER => {
            let agent_id = buf.read_u16_le()?;
            let position = decode_position(buf)?;
            let facing = decode_facing(buf)?;
            let name_len = buf.read_u16_le()? as usize;
            let name = buf.read_string(name_len)?;
            let level = buf.read_u16_le()?;
            let health = Health {
                current: buf.read_u32_le()?,
                max: buf.read_u32_le()?,
            };
            let mana = Mana {
                current: buf.read_u32_le()?,
                max: buf.read_u32_le()?,
            };
            let outfit = buf.read_u16_le()?;
            let color1 = buf.read_u8()?;
            let color2 = buf.read_u8()?;
            let color3 = buf.read_u8()?;
            let color4 = buf.read_u8()?;
            let speed = buf.read_u16_le()?;
            let capacity = buf.read_u32_le()?;
            let inventory_head = decode_optional_item(buf)?;
            let inventory_amulet = decode_optional_item(buf)?;
            let inventory_backpack = decode_optional_item(buf)?;
            let inventory_chest = decode_optional_item(buf)?;
            let inventory_right_hand = decode_optional_item(buf)?;
            let inventory_left_hand = decode_optional_item(buf)?;
            let inventory_legs = decode_optional_item(buf)?;
            let inventory_feet = decode_optional_item(buf)?;
            let inventory_ring = decode_optional_item(buf)?;
            let inventory_trinket = decode_optional_item(buf)?;
            Ok(ServerMessage::DescribePlayer {
                agent_id,
                position,
                facing,
                name,
                level,
                health,
                mana,
                outfit: (outfit, (color1, color2, color3, color4)),
                speed,
                capacity,
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
            })
        }
        MSG_DESCRIBE_MAP => {
            let center = decode_position(buf)?;
            let floor = buf.read_u8()?;
            let mut tiles = Box::new([[None; STACK_MAX_VISIBLE_ITEMS]; TILES_X * TILES_Y]);
            for tile in tiles.iter_mut() {
                *tile = decode_tile(buf)?;
            }
            Ok(ServerMessage::DescribeMap {
                tiles,
                floor,
                center,
            })
        }
        MSG_TILE_CHANGED => {
            let position = decode_position(buf)?;
            let items = Box::new(decode_tile(buf)?);
            Ok(ServerMessage::TileChanged { position, items })
        }
        MSG_PLAYER_WALK_ACK => {
            let position = decode_position(buf)?;
            let mut floor_tiles: Vec<(u8, Box<[ItemStack]>)> = Vec::new();
            let mut floor = buf.read_u8()?;
            while floor != 0xFF {
                let tiles_len = buf.read_u8()?;
                let mut tiles = Vec::with_capacity(tiles_len as usize);
                for _ in 0..tiles_len {
                    tiles.push(decode_tile(buf)?);
                }
                floor_tiles.push((floor, tiles.into_boxed_slice()));
                floor = buf.read_u8()?;
            }
            Ok(ServerMessage::PlayerWalkAck {
                position,
                tiles: floor_tiles,
            })
        }
        MSG_PLAYER_POS => {
            let position = decode_position(buf)?;
            Ok(ServerMessage::PlayerPosition { position })
        }
        MSG_TEXT_MESSAGE => {
            let text_len = buf.read_u16_le()? as usize;
            let text = buf.read_string(text_len)?;
            let message_type = decode_text_type(buf.read_u8()?)?;
            Ok(ServerMessage::TextMessage { text, message_type })
        }
        MSG_OPEN_CONTAINER => {
            let container_id = buf.read_u16_le()?;
            let capacity = buf.read_u8()?;
            let has_parent = buf.read_u8()? != 0;
            let title_len = buf.read_u8()? as usize;
            let title = buf.read_string(title_len)?;
            let items = decode_items(buf)?;
            Ok(ServerMessage::OpenContainer {
                container_id,
                capacity,
                has_parent,
                title,
                items,
            })
        }
        MSG_UPDATE_CONTAINER => {
            let container_id = buf.read_u16_le()?;
            let items = decode_items(buf)?;
            Ok(ServerMessage::UpdateContainer {
                container_id,
                items,
            })
        }
        MSG_CONTAINER_CLOSED => {
            let container_id = buf.read_u16_le()?;
            Ok(ServerMessage::ContainerClosed { container_id })
        }
        MSG_PLAYER_WALK_DENIED => Ok(ServerMessage::PlayerWalkDenied),
        MSG_INVETORY_SLOT_UPDATED => {
            let slot = InventorySlot::from_id(buf.read_u8()?);
            let Some(slot) = slot else {
                return Err(MessageDecodeError::WrongSequence);
            };
            let item_id = decode_optional_item(buf)?;
            Ok(ServerMessage::IventorySlotUpdated { slot, item_id })
        }
        MSG_PLAYER_CAPACITY_UPDATED => {
            let capacity = buf.read_u32_le()?;
            Ok(ServerMessage::PlayerCapacityUpdated { capacity })
        }
        MSG_AGENT_DIRECTION_CHANGED => Ok(ServerMessage::AgentChangedDirection {
            agent_id: buf.read_u16_le()?,
            facing: decode_facing(buf)?,
        }),
        MSG_REMOVE_AGENT => Ok(ServerMessage::RemoveAgent {
            agent_id: buf.read_u16_le()?,
        }),
        MSG_MOVE_AGENT => Ok(ServerMessage::MoveAgent {
            agent_id: buf.read_u16_le()?,
            direction: decode_direction(buf)?,
            from: decode_position(buf)?,
        }),
        MSG_SPAWN_AGENT => {
            let agent_id = buf.read_u16_le()?;
            let position = decode_position(buf)?;
            let facing = decode_facing(buf)?;
            let name_len = buf.read_u16_le()? as usize;
            let name = buf.read_string(name_len)?;
            let health = Health {
                current: buf.read_u32_le()?,
                max: buf.read_u32_le()?,
            };
            let outfit_id = buf.read_u16_le()?;
            let color1 = buf.read_u8()?;
            let color2 = buf.read_u8()?;
            let color3 = buf.read_u8()?;
            let color4 = buf.read_u8()?;
            let speed = buf.read_u16_le()?;
            Ok(ServerMessage::SpawnAgent {
                agent_id,
                outfit: (outfit_id, (color1, color2, color3, color4)),
                position,
                facing,
                name,
                health,
                speed,
            })
        }
        MSG_TELEPORT_AGENT => Ok(ServerMessage::TeleportAgent {
            agent_id: buf.read_u16_le()?,
            position: decode_position(buf)?,
        }),
        MSG_CHAT_MESSAGE => {
            let author = buf.read_u16_le()?;
            let message_type = decode_chat_message_type(buf.read_u8()?)?;
            let channel = buf.read_u16_le()?;
            let text_len = buf.read_u16_le()? as usize;
            let text = buf.read_string(text_len)?;
            Ok(ServerMessage::ChatMessage {
                author,
                message_type,
                channel,
                text,
            })
        }
        MSG_CHANNEL_LIST => {
            let count = buf.read_u16_le()? as usize;
            let mut channels = Vec::new();
            for _ in 0..count {
                let id = buf.read_u16_le()?;
                let name_len = buf.read_u16_le()? as usize;
                let name = buf.read_string(name_len)?;
                channels.push((id, name));
            }
            Ok(ServerMessage::ChannelList { channels })
        }
        MSG_INTRODUCE_PLAYER => {
            let local_id = buf.read_u16_le()?;
            let name_len = buf.read_u16_le()? as usize;
            let name = buf.read_string(name_len)?;
            Ok(ServerMessage::IntroducePlayer { local_id, name })
        }
        _ => Err(MessageDecodeError::WrongSequence),
    }
}

fn decode_tile(buf: &mut Reader) -> Result<ItemStack, MessageDecodeError> {
    let mut tile = [None; STACK_MAX_VISIBLE_ITEMS];
    let mut i = 0;
    loop {
        let id = buf.read_u16_le()?;
        if id == 0xFFFF {
            break;
        }
        let amount = buf.read_u8()?;
        if i < STACK_MAX_VISIBLE_ITEMS {
            tile[i] = Some((id, amount));
            i += 1;
        }
    }
    Ok(tile)
}

fn decode_items(buf: &mut Reader) -> Result<Box<[Option<(ItemId, u8)>]>, MessageDecodeError> {
    let mut items = Vec::new();
    loop {
        let id = buf.read_u16_le()?;
        if id == 0xFFFF {
            break;
        }
        let amount = buf.read_u8()?;
        items.push(Some((id, amount)));
    }
    Ok(items.into())
}

fn decode_position(buf: &mut Reader) -> Result<Position, MessageDecodeError> {
    Ok(Position {
        x: buf.read_u16_le()?,
        y: buf.read_u16_le()?,
        z: buf.read_u8()?,
    })
}

fn decode_facing(buf: &mut Reader) -> Result<FacingDirection, MessageDecodeError> {
    match buf.read_u8()? {
        1 => Ok(FacingDirection::North),
        2 => Ok(FacingDirection::East),
        3 => Ok(FacingDirection::South),
        4 => Ok(FacingDirection::West),
        _ => Err(MessageDecodeError::WrongSequence),
    }
}

fn decode_text_type(b: u8) -> Result<TextMessageType, MessageDecodeError> {
    match b {
        1 => Ok(TextMessageType::ActionDenied),
        2 => Ok(TextMessageType::Look),
        _ => Err(MessageDecodeError::WrongSequence),
    }
}

fn decode_chat_message_type(b: u8) -> Result<ChatMessageType, MessageDecodeError> {
    match b {
        0x01 => Ok(ChatMessageType::Local),
        0x02 => Ok(ChatMessageType::Private),
        0x03 => Ok(ChatMessageType::Channel),
        _ => Err(MessageDecodeError::WrongSequence),
    }
}

fn decode_optional_item(buf: &mut Reader) -> Result<Option<ItemId>, MessageDecodeError> {
    let item_id = buf.read_u16_le()?;
    if item_id == 0xFFFF {
        Ok(None)
    } else {
        Ok(Some(item_id))
    }
}

fn decode_direction(buf: &mut Reader) -> Result<WalkingDirection, MessageDecodeError> {
    match buf.read_u8()? {
        0x00 => Ok(WalkingDirection::North),
        0x01 => Ok(WalkingDirection::East),
        0x02 => Ok(WalkingDirection::West),
        0x03 => Ok(WalkingDirection::South),
        0x04 => Ok(WalkingDirection::NorthEast),
        0x05 => Ok(WalkingDirection::NorthWest),
        0x06 => Ok(WalkingDirection::SouthEast),
        0x07 => Ok(WalkingDirection::SouthWest),
        _ => Err(MessageDecodeError::WrongSequence),
    }
}

impl Encoder for GameMessageCodec {
    type Item<'a> = ClientMessage;
    type Error = std::io::Error;

    fn encode(&mut self, item: ClientMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let len_offset = dst.len();
        dst.put_u16_le(0); // placeholder for payload length

        match item {
            ClientMessage::Ping => {
                dst.put_u8(CLI_PING);
            }
            ClientMessage::Login { auth_token } => {
                dst.put_u8(CLI_LOGIN);
                dst.put_slice(auth_token.as_bytes());
            }
            ClientMessage::MovePlayer { direction } => {
                dst.put_u8(CLI_MOVE_PLAYER);
                encode_direction(&direction, dst);
            }
            ClientMessage::MoveItem {
                from,
                item_id,
                amount,
                stack_index,
                to,
            } => {
                dst.put_u8(CLI_MOVE_ITEM);
                encode_position(from, dst);
                dst.put_u16_le(item_id);
                dst.put_u8(amount);
                dst.put_u8(stack_index);
                encode_position(to, dst);
            }
            ClientMessage::GetPlayerPosition => {
                dst.put_u8(CLI_GET_PLAYER_POS);
            }
            ClientMessage::UseItem {
                position,
                item_id,
                stack_index,
            } => {
                dst.put_u8(CLI_USE_ITEM);
                encode_position(position, dst);
                dst.put_u16_le(item_id);
                dst.put_u8(stack_index);
            }
            ClientMessage::CloseContainer { container_id } => {
                dst.put_u8(CLI_CLOSE_CONTAINER);
                dst.put_u16_le(container_id);
            }
            ClientMessage::OpenParentContainer { container_id } => {
                dst.put_u8(CLI_OPEN_PARENT_CONTAINER);
                dst.put_u16_le(container_id);
            }
            ClientMessage::ChangeDirection { direction } => {
                dst.put_u8(CLI_CHANGE_DIRECTION);
                encode_facing(&direction, dst);
            }
            ClientMessage::Logout => dst.put_u8(CLI_LOGOUT),
            ClientMessage::UseItemWith {
                source,
                source_item_id,
                source_index,
                target,
                target_item_id,
                target_index,
            } => {
                dst.put_u8(CLI_USE_ITEM_WITH);
                encode_position(source, dst);
                dst.put_u16_le(source_item_id);
                dst.put_u8(source_index);
                encode_position(target, dst);
                dst.put_u16_le(target_item_id);
                dst.put_u8(target_index);
            }
            ClientMessage::Look { position } => {
                dst.put_u8(CLI_LOOK);
                encode_position(position, dst);
            }
            ClientMessage::Say {
                message,
                message_type,
                target,
            } => {
                dst.put_u8(CLI_SAY);
                dst.put_u8(encode_chat_message_type(message_type));
                dst.put_u16_le(target);
                // Trailing: the server derives the length from the frame, matching
                // CLI_LOGIN. Do not write an inline length here.
                dst.put_slice(message.as_bytes());
            }
            ClientMessage::RequestChannels => dst.put_u8(CLI_REQUEST_CHANNELS),
            ClientMessage::OpenChannel { channel } => {
                dst.put_u8(CLI_OPEN_CHANNEL);
                dst.put_u16_le(channel);
            }
            ClientMessage::CloseChannel { channel } => {
                dst.put_u8(CLI_CLOSE_CHANNEL);
                dst.put_u16_le(channel);
            }
            ClientMessage::OpenPmChat { name } => {
                dst.put_u8(CLI_OPEN_PM_CHAT);
                dst.put_slice(name.as_bytes());
            }
        }

        let payload_len = (dst.len() - len_offset - 2) as u16;
        dst[len_offset..len_offset + 2].copy_from_slice(&payload_len.to_le_bytes());

        Ok(())
    }
}

fn encode_position(pos: Position, dst: &mut BytesMut) {
    dst.put_u16_le(pos.x);
    dst.put_u16_le(pos.y);
    dst.put_u8(pos.z);
}

fn encode_direction(d: &WalkingDirection, dst: &mut BytesMut) {
    let value = match d {
        WalkingDirection::North => 0x00,
        WalkingDirection::East => 0x01,
        WalkingDirection::West => 0x02,
        WalkingDirection::South => 0x03,
        WalkingDirection::NorthEast => 0x04,
        WalkingDirection::NorthWest => 0x05,
        WalkingDirection::SouthEast => 0x06,
        WalkingDirection::SouthWest => 0x07,
    };
    dst.put_u8(value);
}

fn encode_facing(d: &FacingDirection, dst: &mut BytesMut) {
    let value = match d {
        FacingDirection::North => 1,
        FacingDirection::East => 2,
        FacingDirection::South => 3,
        FacingDirection::West => 4,
    };
    dst.put_u8(value);
}

fn encode_chat_message_type(t: ChatMessageType) -> u8 {
    match t {
        ChatMessageType::Local => 0x01,
        ChatMessageType::Private => 0x02,
        ChatMessageType::Channel => 0x03,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asynchronous_codec::Encoder;

    /// Encodes `msg` and returns the payload with the 2-byte length prefix stripped,
    /// after asserting the prefix matches the real payload length.
    fn payload_of(msg: ClientMessage) -> Vec<u8> {
        let mut codec = GameMessageCodec {};
        let mut buf = BytesMut::new();
        codec.encode(msg, &mut buf).unwrap();
        let declared = u16::from_le_bytes([buf[0], buf[1]]) as usize;
        assert_eq!(
            declared,
            buf.len() - 2,
            "length prefix must cover the payload"
        );
        buf[2..].to_vec()
    }

    #[test]
    fn say_encodes_type_target_then_trailing_message() {
        let payload = payload_of(ClientMessage::Say {
            message: "hello".to_owned(),
            message_type: ChatMessageType::Local,
            target: 0,
        });
        assert_eq!(payload[0], CLI_SAY);
        assert_eq!(payload[1], 0x01, "Local");
        assert_eq!(u16::from_le_bytes([payload[2], payload[3]]), 0);
        assert_eq!(
            &payload[4..],
            b"hello",
            "message is trailing, no inline length"
        );
    }

    #[test]
    fn say_carries_the_channel_in_target() {
        let payload = payload_of(ClientMessage::Say {
            message: "hi".to_owned(),
            message_type: ChatMessageType::Channel,
            target: 7,
        });
        assert_eq!(payload[1], 0x03, "Channel");
        assert_eq!(u16::from_le_bytes([payload[2], payload[3]]), 7);
    }

    #[test]
    fn private_say_uses_type_two() {
        let payload = payload_of(ClientMessage::Say {
            message: "psst".to_owned(),
            message_type: ChatMessageType::Private,
            target: 3,
        });
        assert_eq!(payload[1], 0x02, "Private");
        assert_eq!(u16::from_le_bytes([payload[2], payload[3]]), 3);
    }

    #[test]
    fn channel_control_messages_encode() {
        assert_eq!(
            payload_of(ClientMessage::RequestChannels),
            vec![CLI_REQUEST_CHANNELS]
        );

        let payload = payload_of(ClientMessage::OpenChannel { channel: 2 });
        assert_eq!(payload[0], CLI_OPEN_CHANNEL);
        assert_eq!(u16::from_le_bytes([payload[1], payload[2]]), 2);

        let payload = payload_of(ClientMessage::CloseChannel { channel: 2 });
        assert_eq!(payload[0], CLI_CLOSE_CHANNEL);
        assert_eq!(u16::from_le_bytes([payload[1], payload[2]]), 2);
    }

    #[test]
    fn open_pm_chat_encodes_a_trailing_name() {
        let payload = payload_of(ClientMessage::OpenPmChat {
            name: "Rizael".to_owned(),
        });
        assert_eq!(payload[0], CLI_OPEN_PM_CHAT);
        assert_eq!(&payload[1..], b"Rizael");
    }

    use asynchronous_codec::Decoder;

    /// Wraps `payload` in the 2-byte length prefix the codec expects.
    fn frame(payload: &[u8]) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        buf.extend_from_slice(payload);
        buf
    }

    #[test]
    fn decodes_a_chat_message() {
        let mut payload = vec![MSG_CHAT_MESSAGE];
        payload.extend_from_slice(&3u16.to_le_bytes()); // author
        payload.push(0x03); // Channel
        payload.extend_from_slice(&7u16.to_le_bytes()); // channel
        payload.extend_from_slice(&5u16.to_le_bytes()); // len
        payload.extend_from_slice(b"hello");

        let mut codec = GameMessageCodec {};
        let mut buf = frame(&payload);
        match codec.decode(&mut buf).unwrap().unwrap() {
            ServerMessage::ChatMessage {
                author,
                message_type,
                channel,
                text,
            } => {
                assert_eq!(author, 3);
                assert!(matches!(message_type, ChatMessageType::Channel));
                assert_eq!(channel, 7);
                assert_eq!(text, "hello");
            }
            other => panic!("expected ChatMessage, got {other:?}"),
        }
        assert!(buf.is_empty(), "the frame must be fully consumed");
    }

    #[test]
    fn decodes_introduce_player() {
        let mut payload = vec![MSG_INTRODUCE_PLAYER];
        payload.extend_from_slice(&9u16.to_le_bytes());
        payload.extend_from_slice(&6u16.to_le_bytes());
        payload.extend_from_slice(b"Rizael");

        let mut codec = GameMessageCodec {};
        let mut buf = frame(&payload);
        match codec.decode(&mut buf).unwrap().unwrap() {
            ServerMessage::IntroducePlayer { local_id, name } => {
                assert_eq!(local_id, 9);
                assert_eq!(name, "Rizael");
            }
            other => panic!("expected IntroducePlayer, got {other:?}"),
        }
        assert!(buf.is_empty());
    }

    /// Three entries with different-length names and a non-contiguous third id. A
    /// single-entry test cannot tell a correct loop from one that stops after the
    /// first entry, or that writes an index instead of the real id.
    #[test]
    fn decodes_every_entry_of_a_channel_list() {
        let entries: [(u16, &[u8]); 3] = [(1, b"World Chat"), (2, b"Advertising"), (7, b"Help")];
        let mut payload = vec![MSG_CHANNEL_LIST];
        payload.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        for (id, name) in entries.iter() {
            payload.extend_from_slice(&id.to_le_bytes());
            payload.extend_from_slice(&(name.len() as u16).to_le_bytes());
            payload.extend_from_slice(name);
        }

        let mut codec = GameMessageCodec {};
        let mut buf = frame(&payload);
        match codec.decode(&mut buf).unwrap().unwrap() {
            ServerMessage::ChannelList { channels } => {
                assert_eq!(channels.len(), 3);
                assert_eq!(channels[0], (1, "World Chat".to_owned()));
                assert_eq!(channels[1], (2, "Advertising".to_owned()));
                assert_eq!(channels[2], (7, "Help".to_owned()));
            }
            other => panic!("expected ChannelList, got {other:?}"),
        }
        assert!(buf.is_empty(), "the frame must be fully consumed");
    }

    /// A length field larger than the payload used to index straight past the
    /// buffer and panic, killing the connection task.
    #[test]
    fn rejects_a_length_field_past_the_end_of_the_payload() {
        let mut payload = vec![MSG_INTRODUCE_PLAYER];
        payload.extend_from_slice(&9u16.to_le_bytes());
        payload.extend_from_slice(&600u16.to_le_bytes()); // claims 600 bytes of name
        payload.extend_from_slice(b"Rizael");

        let mut codec = GameMessageCodec {};
        let mut buf = frame(&payload);
        assert!(matches!(
            codec.decode(&mut buf),
            Err(MessageDecodeError::Truncated)
        ));
        assert!(buf.is_empty(), "the bad frame is consumed, not left behind");
    }

    /// A tile whose 0xFFFF terminator never arrives used to read on into the
    /// next frame's bytes; now it stops at the payload boundary.
    #[test]
    fn rejects_a_tile_with_no_terminator() {
        let mut payload = vec![MSG_TILE_CHANGED];
        payload.extend_from_slice(&1u16.to_le_bytes()); // x
        payload.extend_from_slice(&2u16.to_le_bytes()); // y
        payload.push(7); // z
        payload.extend_from_slice(&100u16.to_le_bytes()); // item id
        payload.push(1); // amount, and then nothing

        let mut codec = GameMessageCodec {};
        let mut buf = frame(&payload);
        buf.extend_from_slice(&frame(&[MSG_PLAYER_WALK_DENIED])); // the next frame

        assert!(matches!(
            codec.decode(&mut buf),
            Err(MessageDecodeError::Truncated)
        ));
        // Framing survives: the following message still decodes.
        assert!(matches!(
            codec.decode(&mut buf).unwrap().unwrap(),
            ServerMessage::PlayerWalkDenied
        ));
    }

    /// Silent drift is worse than a disconnect: a decoder that stops early
    /// would otherwise misread every field of a version-skewed message.
    #[test]
    fn rejects_a_payload_with_unread_bytes() {
        let mut payload = vec![MSG_CONTAINER_CLOSED];
        payload.extend_from_slice(&4u16.to_le_bytes());
        payload.push(0xAB); // one byte too many

        let mut codec = GameMessageCodec {};
        let mut buf = frame(&payload);
        assert!(matches!(
            codec.decode(&mut buf),
            Err(MessageDecodeError::TrailingBytes(1))
        ));
    }

    #[test]
    fn a_truncated_frame_waits_for_more_bytes() {
        let mut codec = GameMessageCodec {};
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&10u16.to_le_bytes()); // declares 10 bytes
        buf.extend_from_slice(&[MSG_PLAYER_POS, 1, 0]); // only 3 arrived

        assert!(codec.decode(&mut buf).unwrap().is_none());
        assert_eq!(buf.len(), 5, "the partial frame is left buffered");
    }

    #[test]
    fn rejects_an_unknown_chat_message_type() {
        let mut payload = vec![MSG_CHAT_MESSAGE];
        payload.extend_from_slice(&0u16.to_le_bytes());
        payload.push(0x09); // not a valid type
        payload.extend_from_slice(&0u16.to_le_bytes());
        payload.extend_from_slice(&0u16.to_le_bytes());

        let mut codec = GameMessageCodec {};
        let mut buf = frame(&payload);
        assert!(matches!(
            codec.decode(&mut buf),
            Err(MessageDecodeError::WrongSequence)
        ));
    }
}
