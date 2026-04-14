use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::core::{GameState, InstanceManager};

mod colors;
mod components;
mod hud;
mod instancing;
mod material;
pub mod movement;

pub use crate::actor::components::*;
pub use crate::actor::instancing::{spawn_actor, LoadedMaterials};
pub use crate::actor::material::{ActorInstance, ActorMaterial};
pub use crate::actor::movement::{MoveActor, Moving, UpdateElevation};

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
            .add_systems(
                Update,
                (
                    hud::update_display_name_health_state,
                    hud::update_display_name_color,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                (
                    hud::update_hud_bar_ratios,
                    hud::update_hud_bar_health_state.after(hud::update_hud_bar_ratios),
                    hud::update_hud_bar_colors.after(hud::update_hud_bar_health_state),
                    hud::resize_hud_fill.after(hud::update_hud_bar_ratios),
                )
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                hud::update_hud_positions.run_if(in_state(GameState::InGame)),
            )
            .add_observer(instancing::on_remove_actor)
            .add_observer(movement::on_actor_move)
            .add_observer(movement::on_actor_change_direction)
            .add_observer(movement::on_update_elevation);

        #[cfg(feature = "debug")]
        app.add_systems(Update, (instancing::actor_rect,));
    }
}
