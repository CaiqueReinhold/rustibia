use bevy::{prelude::*, sprite_render::Material2dPlugin};

use crate::{
    core::{InstanceManager, State},
    items::{map_stack::ItemStacks, material::ItemInstance},
};

pub mod container;
mod item;
mod map_stack;
mod material;
mod ui_item;

pub use container::{LootContainerUI, OpenContainer};
pub use item::{Item, ItemConfig};
pub use ui_item::{ItemDragEnded, ItemDragStarted};

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<material::ItemMaterial>::default())
            .init_resource::<InstanceManager<ItemInstance>>()
            .init_resource::<ItemStacks>()
            .add_systems(Startup, map_stack::init_material_buffer)
            .add_systems(
                Update,
                (
                    map_stack::upload_instance_buffer,
                    ui_item::move_dragged_item,
                    container::container_content_changed,
                )
                    .run_if(in_state(State::InGame)),
            )
            .add_observer(map_stack::on_tile_changed)
            .add_observer(map_stack::on_remove_item)
            .add_observer(ui_item::item_drag_started)
            .add_observer(ui_item::item_drag_ended)
            .add_observer(container::on_open_container);
    }
}
