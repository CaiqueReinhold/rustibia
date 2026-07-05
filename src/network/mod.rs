use bevy::prelude::*;

use crate::{core::GameState, network::systems::ConnectionState};

pub mod events;
mod messages;
mod systems;

pub use messages::{ClientMessage, ItemStack, ServerMessage};
pub use systems::{LoginCredentials, SendMessage};
pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(systems::on_connect)
            .add_observer(systems::on_send_message)
            .add_observer(systems::on_login_error_cleanup)
            .add_observer(systems::on_connection_lost_cleanup)
            .add_systems(
                Update,
                systems::receive_messages.run_if(resource_exists::<ConnectionState>),
            )
            .add_systems(OnEnter(GameState::Connecting), systems::connect);
    }
}
