use bevy::prelude::*;

use crate::core::GameState;

pub mod events;
pub mod minimap;
pub mod minimap_ui;
mod position;
mod storage;

pub use crate::map::position::Position;
pub use crate::map::storage::Map;
pub use minimap::MinimapData;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_plugins(Material2dPlugin::<material::TerrainMaterial>::default())
            // .init_resource::<chunks::LoadedChunks>()
            // .init_resource::<chunks::LoadedMaterials>()
            .init_resource::<storage::Map>()
            .init_resource::<minimap::MinimapData>()
            .init_resource::<minimap::SaveTimer>()
            .add_plugins(minimap_ui::MinimapPlugin)
            .add_observer(events::on_describe_map)
            .add_observer(events::on_tile_changed)
            .add_systems(Startup, minimap::load_from_disk)
            .add_systems(OnEnter(GameState::InGame), storage::init_map)
            .add_systems(
                FixedUpdate,
                minimap::save_dirty_chunks.run_if(in_state(GameState::InGame)),
            );
        #[cfg(feature = "debug")]
        app.add_systems(Update, chunks::draw_tile_grid);
    }
}
