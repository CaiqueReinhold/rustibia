use bevy::prelude::*;

use crate::core::GameState;

pub mod events;
mod position;
mod storage;

pub use crate::map::position::Position;
pub use crate::map::storage::Map;
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_plugins(Material2dPlugin::<material::TerrainMaterial>::default())
            // .init_resource::<chunks::LoadedChunks>()
            // .init_resource::<chunks::LoadedMaterials>()
            .init_resource::<storage::Map>()
            // .add_systems(
            // Update,
            // chunks::update_visible_chunks
            // .run_if(in_state(GameState::InGame).and(resource_changed::<Map>)),
            // )
            .add_observer(events::on_describe_map)
            .add_observer(events::on_tile_changed)
            .add_systems(OnEnter(GameState::InGame), storage::init_map);
        #[cfg(feature = "debug")]
        app.add_systems(Update, chunks::draw_tile_grid);
    }
}
