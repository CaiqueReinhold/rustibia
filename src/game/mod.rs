pub mod hud;
pub mod map;
pub mod player;

use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, map::spawn_map)
            .add_systems(Update, map::debug_print_map_loaded)
            .add_plugins(player::PlayerPlugin);
    }
}
