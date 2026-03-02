use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::core::State;

mod actions;
mod actor;
mod assets;
mod colors;
mod hud;
mod material;
mod movement;
mod player;

pub use crate::actor::assets::*;
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
                (actor::update_actor_instances, actor::upload_instance_buffer)
                    .chain()
                    .run_if(in_state(State::InGame)),
            )
            .add_systems(OnEnter(State::InGame), player::spawn_player)
            .add_systems(PreUpdate, player::read_player_input)
            .add_systems(Update, (movement::move_actor, player::center_on_player))
            .add_systems(Update, (actor::actor_rect, player::show_pos))
            .add_observer(actor::on_remove_actor)
            .add_observer(actions::on_player_move)
            .add_observer(actions::on_player_change_direction);
    }
}
