use bevy::{prelude::*, sprite_render::Material2dPlugin};

use crate::{
    core::{GameState, InstanceManager},
    items::{instancing::ItemStacks, material::ItemInstance},
};

pub mod container;
mod instancing;
mod item;
mod material;
mod ui_item;

pub use container::{ContainerId, LootContainerUI};
pub use instancing::ChangedTileQueue;
pub use item::{Item, ItemConfig, ItemFlag, ItemId};
pub use ui_item::{ItemDragEnded, ItemDragStarted, ItemMoveCanceled, ItemMoveConfirmed};

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<material::ItemMaterial>::default())
            .init_resource::<InstanceManager<ItemInstance>>()
            .init_resource::<ItemStacks>()
            .init_resource::<ChangedTileQueue>()
            .add_systems(Startup, instancing::init_material_buffer)
            .add_systems(
                Update,
                (
                    instancing::process_tile_changed,
                    ui_item::move_dragged_item,
                    container::container_content_changed,
                )
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                instancing::upload_instance_buffer.run_if(in_state(GameState::InGame)),
            )
            .add_observer(instancing::on_remove_item)
            .add_observer(ui_item::item_drag_started)
            .add_observer(ui_item::item_drag_ended)
            .add_observer(ui_item::item_move_confirmed)
            .add_observer(ui_item::item_move_canceled)
            .add_observer(container::on_open_container)
            .add_observer(container::on_update_container)
            .add_observer(container::on_container_closed)
            .add_observer(container::on_container_ui_closed);
    }
}
