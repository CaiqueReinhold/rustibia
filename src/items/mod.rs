use bevy::{prelude::*, sprite_render::Material2dPlugin};

use crate::{
    core::InstanceManager,
    items::{item::ItemStacks, material::ItemInstance},
};

mod item;
mod material;

pub use item::{Item, ItemConfig};

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<material::ItemMaterial>::default())
            .init_resource::<InstanceManager<ItemInstance>>()
            .init_resource::<ItemStacks>()
            .add_systems(Startup, item::init_material_buffer)
            .add_systems(Update, item::upload_instance_buffer)
            .add_observer(item::on_tile_changed)
            .add_observer(item::on_remove_item);
    }
}
