use bevy::{prelude::*, sprite_render::Material2dPlugin};

use crate::{
    core::InstanceManager,
    items::{map_stack::ItemStacks, material::ItemInstance},
};

mod item;
mod map_stack;
mod material;
mod ui_item;

pub use item::{Item, ItemConfig};
pub use ui_item::{ItemDragEnded, ItemDragOrigin, ItemDragStarted};

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
                ),
            )
            .add_observer(map_stack::on_tile_changed)
            .add_observer(map_stack::on_remove_item)
            .add_observer(ui_item::item_drag_started)
            .add_observer(ui_item::item_drag_ended);
    }
}
