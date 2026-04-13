use bevy::{prelude::*, sprite_render::Material2dPlugin};

use crate::{
    core::{GameState, InstanceManager},
    items::{instancing::ItemStacks, material::ItemInstance},
};

mod container;
mod events;
mod instancing;
pub mod inventory;
mod item;
mod material;
mod ui_item;

pub use container::{ContainerId, LootContainerUI};
pub use events::*;
pub use instancing::ChangedTileQueue;
pub use item::{InventorySlot, Item, ItemConfig, ItemFlag, ItemId, ItemPlacement};

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<material::ItemMaterial>::default())
            .init_resource::<InstanceManager<ItemInstance>>()
            .init_resource::<ItemStacks>()
            .init_resource::<ChangedTileQueue>()
            .add_systems(Startup, instancing::init_material_buffer)
            .add_systems(
                OnEnter(GameState::InGame),
                inventory::spawn_inventory_ui.after(crate::game_ui::spawn_main_ui),
            )
            .add_systems(
                Update,
                (
                    instancing::process_tile_changed,
                    ui_item::animate_ui_items,
                    ui_item::move_dragged_item,
                    container::container_content_changed,
                    inventory::update_inventory_ui,
                    inventory::update_capacity,
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
            .add_observer(container::on_container_closed_by_server)
            .add_observer(container::on_container_ui_closed)
            .add_observer(container::on_open_parent_container);
    }
}
