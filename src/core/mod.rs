use bevy::prelude::*;

mod assets;
mod instances;
mod sprite;

pub use crate::core::assets::*;
pub use crate::core::instances::*;
pub use crate::core::sprite::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, States, Default)]
pub enum State {
    #[default]
    LoadingAssets,
    // LoginScreen,
    // Connecting,
    InGame,
}

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, assets::start_load_tasks)
            .add_systems(
                FixedUpdate,
                (
                    assets::pool_load_task.run_if(resource_exists::<LoadTasks>),
                    assets::pool_all_assets_loaded.run_if(resource_exists::<GameAssetsLoaded>),
                ),
            );
    }
}
