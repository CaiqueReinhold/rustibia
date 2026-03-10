use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::core::State;

mod assets;
mod chunks;
mod events;
mod material;
mod position;
mod storage;

pub use crate::map::assets::read_map_config;
pub use crate::map::events::TileChanged;
pub use crate::map::position::TilePosition;
pub use crate::map::storage::Map;
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<material::TerrainMaterial>::default())
            .init_resource::<chunks::LoadedChunks>()
            .init_resource::<chunks::LoadedMaterials>()
            .add_observer(chunks::update_visible_chunks)
            .add_systems(
                FixedUpdate,
                (chunks::player_chunk_changed).run_if(in_state(State::InGame)),
            );
        #[cfg(feature = "debug")]
        app.add_systems(Update, chunks::draw_tile_grid);
    }
}
