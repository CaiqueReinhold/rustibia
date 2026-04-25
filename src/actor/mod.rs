use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::core::{AnimationSet, GameState, InstanceManager};

// mod colors;
mod components;
mod events;
mod hud;
mod instancing;
mod material;
pub mod movement;

pub use crate::actor::components::*;
pub use crate::actor::instancing::{LoadedMaterials, spawn_actor};
pub use crate::actor::material::{ActorInstance, ActorMaterial};
pub use crate::actor::movement::{MoveActor, MoveQueue, Moving, UpdateElevation};

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
                    movement::process_actor_move_queues,
                    movement::teleport_agents,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                instancing::set_actor_animation_state.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                instancing::update_actor_instances
                    .after(AnimationSet)
                    .after(instancing::set_actor_animation_state)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                instancing::upload_instance_buffer.run_if(in_state(GameState::InGame)),
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
            .add_observer(movement::on_actor_move)
            .add_observer(movement::on_actor_change_direction)
            .add_observer(movement::on_update_elevation)
            .add_observer(movement::on_teleport_agent)
            .add_observer(events::on_spawn_agent)
            .add_observer(events::on_move_agent)
            .add_observer(events::on_remove_actor);

        #[cfg(feature = "debug")]
        app.add_systems(Update, (instancing::actor_rect,));
    }
}
