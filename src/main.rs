use bevy::prelude::*;
use bevy::window::PresentMode;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

mod actor;
mod camera;
mod conf;
mod core;
mod main_ui;
mod map;

use crate::core::{GameAssetsLoaded, State};

fn main() {
    App::new()
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Tibra".into(),
                        name: Some("bevy.app".into()),
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(
            Startup,
            (camera::spawn_ui_camera, camera::spawn_game_camera),
        )
        .add_plugins((
            core::CorePlugin,
            actor::ActorPlugin,
            map::MapPlugin,
            main_ui::UiPlugin,
        ))
        .init_state::<State>()
        .init_resource::<GameAssetsLoaded>()
        .run();
}
