use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::actor::material::ActorInstance;
use crate::core::InstanceManager;
use crate::core::State;

mod actions;
mod assets;
mod colors;
mod hud;
mod material;
mod movement;
mod player;
mod spawning;

pub use crate::actor::assets::*;
pub use crate::actor::hud::*;
pub use crate::actor::player::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FacingDirection {
    #[default]
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl From<FacingDirection> for u32 {
    fn from(value: FacingDirection) -> Self {
        value as u32
    }
}

impl From<FacingDirection> for usize {
    fn from(value: FacingDirection) -> Self {
        value as usize
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Mounted {
    #[default]
    Unmounted = 0,
    Mounted = 1,
}

impl From<Mounted> for u32 {
    fn from(value: Mounted) -> Self {
        value as u32
    }
}

#[derive(Component, Debug, Default)]
pub struct Actor {
    // pub outfit_id: u32,
    pub direction: FacingDirection,
    pub addons: u32,
    pub mounted: Mounted,
    pub color_head: u32,
    pub color_body: u32,
    pub color_legs: u32,
    pub color_feet: u32,
    pub speed: u32,
    pub box_size: [f32; 2],
    pub boxes: [[Rect; 4]; 2],
    pub phase_counts: [u32; 2],
}

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<material::ActorMaterial>::default())
            .init_resource::<InstanceManager<ActorInstance>>()
            .add_systems(Startup, spawning::init_instances_buffer)
            .add_systems(
                Update,
                (
                    spawning::update_actor_instances,
                    spawning::upload_instance_buffer,
                )
                    .chain()
                    .run_if(in_state(State::InGame)),
            )
            .add_systems(OnEnter(State::InGame), player::spawn_player)
            .add_systems(PreUpdate, player::read_player_input)
            .add_systems(Update, (movement::move_actor, player::center_on_player))
            .add_observer(spawning::on_remove_actor)
            .add_observer(actions::on_player_move)
            .add_observer(actions::on_player_change_direction);

        #[cfg(feature = "debug")]
        app.add_systems(Update, (spawning::actor_rect,));
    }
}
