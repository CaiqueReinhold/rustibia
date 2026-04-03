use std::sync::Arc;

use bevy::prelude::*;

use crate::items::{ContainerId, Item, ItemPlacement};

#[derive(Event)]
pub struct ItemDragStarted {
    pub item: Arc<Item>,
    pub origin: ItemPlacement,
}

#[derive(Event)]
pub struct ItemDragEnded;

#[derive(Event)]
pub struct ItemMoveCanceled;

#[derive(Event)]
pub struct ItemMoveConfirmed;

#[derive(Event)]
pub struct OpenParentContainer {
    pub container_id: ContainerId,
}
