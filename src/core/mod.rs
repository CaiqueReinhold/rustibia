use bevy::prelude::*;

mod assets;
mod instances;
mod items;
mod sprite;
mod systems;
mod text;

pub use assets::*;
pub use instances::*;
pub use items::ItemConfigs;
pub use sprite::*;
pub use systems::PingState;
pub use text::TextMessageType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, States, Default)]
pub enum GameState {
    #[default]
    LoadingAssets,
    // LoginScreen,
    Connecting,
    InGame,
}

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<systems::PingState>()
            .add_systems(Startup, assets::start_load_tasks)
            .add_systems(
                FixedUpdate,
                (
                    assets::pool_load_task.run_if(resource_exists::<LoadTasks>),
                    assets::pool_all_assets_loaded.run_if(resource_exists::<GameAssetsLoaded>),
                    systems::send_ping.run_if(in_state(GameState::InGame)),
                ),
            )
            .add_observer(systems::receive_pong)
            .add_observer(text::on_text_message);
    }
}
