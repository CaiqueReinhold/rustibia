use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::core::{GameState, InstanceManager};

mod colors;
mod components;
mod hud;
mod instancing;
mod material;
mod movement;

pub use crate::actor::components::{FacingDirection, WalkingDirection};
pub use crate::actor::hud::*;
pub use crate::actor::instancing::{spawn_actor, LoadedMaterials};
pub use crate::actor::material::{ActorInstance, ActorMaterial};
pub use crate::actor::movement::{ActorChangeDirection, MoveActor, Moving};

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<material::ActorMaterial>::default())
            .init_resource::<InstanceManager<ActorInstance>>()
            .add_systems(Startup, instancing::init_instances_buffer)
            .add_systems(
                Update,
                (
                    movement::move_actor,
                    instancing::update_actor_instances,
                    instancing::upload_instance_buffer,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame)),
            )
            .add_observer(instancing::on_remove_actor)
            .add_observer(movement::on_actor_move)
            .add_observer(movement::on_actor_change_direction);

        #[cfg(feature = "debug")]
        app.add_systems(Update, (instancing::actor_rect,));
    }
}
