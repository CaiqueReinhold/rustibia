use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::core::State;

mod assets;
mod chunks;
mod map;
mod material;
mod position;

pub use crate::map::assets::read_map_config;
pub use crate::map::map::Map;
pub use crate::map::position::TilePosition;
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
            )
            .add_systems(Update, chunks::draw_tile_grid);
    }
}
