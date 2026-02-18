use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

mod actor;
mod assets;
mod colors;
mod hud;
mod material;
mod movement;
mod player;

pub use crate::actor::hud::*;
pub use crate::actor::player::*;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<material::ActorMaterial>::default())
            .init_resource::<actor::LoadedMaterials>()
            .add_systems(Startup, actor::init_instances_buffer)
            .add_systems(
                Update,
                (actor::update_actor_instances, actor::upload_instance_buffer).chain(),
            )
            .add_observer(actor::on_spawn_actor)
            .add_observer(actor::on_remove_actor);
    }
}
