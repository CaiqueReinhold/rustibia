use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Camera, ClearColorConfig, OrthographicProjection, ScalingMode};
use bevy::prelude::*;
use bevy::window::PresentMode;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use crate::data::State;

mod conf;
mod data;
mod game;
mod ui;

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
        .add_systems(Startup, (spawn_ui_camera, spawn_game_camera))
        .add_plugins((data::AssetsLoaderPlugin, game::GamePlugin, ui::UiPlugin))
        .init_state::<State>()
        .run();
}

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(1),
        IsDefaultUiCamera,
    ));
}

fn spawn_game_camera(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: conf::viewport::GAME_VIEW_WIDTH,
        height: conf::viewport::GAME_VIEW_HEIGHT,
    };
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        Projection::Orthographic(projection),
        ui::gameview::GameCamera,
        Transform::default(),
        GlobalTransform::default(),
        Name::new("Game Camera"),
    ));
}
