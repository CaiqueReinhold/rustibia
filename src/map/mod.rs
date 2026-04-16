use bevy::prelude::*;

use crate::core::GameState;

pub mod events;
mod floors;
pub mod minimap;
pub mod minimap_ui;
mod position;
mod storage;

pub use crate::map::position::Position;
pub use crate::map::storage::Map;
pub use floors::FloorEntities;
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
            .add_systems(Startup, (minimap::load_from_disk, floors::setup_floors))
            .add_systems(OnEnter(GameState::InGame), storage::init_map)
            .add_systems(PostUpdate, floors::update_floors_visibility)
            .add_systems(
                FixedUpdate,
                minimap::save_dirty_chunks.run_if(in_state(GameState::InGame)),
            );
        #[cfg(feature = "debug")]
        app.add_systems(Update, draw_tile_grid);
    }
}

#[cfg(feature = "debug")]
fn draw_tile_grid(
    camera_q: Single<&Transform, With<crate::camera::GameCamera>>,
    mut gizmos: Gizmos,
) {
    use crate::conf::map::{TILE_SIZE, TILES_X, TILES_Y};

    let cam = camera_q.translation.truncate();

    // Snap to tile grid: find top-left corner of the visible tile buffer
    let half_w = (TILES_X as f32 * TILE_SIZE) / 2.0;
    let half_h = (TILES_Y as f32 * TILE_SIZE) / 2.0;
    let origin = Vec2::new(
        (cam.x - half_w).round(),
        (cam.y + half_h).round(),
    );

    for ty in 0..TILES_Y {
        for tx in 0..TILES_X {
            let center = Vec2::new(
                origin.x + (tx as f32 + 0.5) * TILE_SIZE,
                origin.y - (ty as f32 + 0.5) * TILE_SIZE,
            );
            gizmos.rect_2d(center, Vec2::splat(TILE_SIZE), Color::srgba(0.0, 1.0, 0.0, 0.25));
        }
    }
}
