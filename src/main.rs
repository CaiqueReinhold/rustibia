use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Camera, ClearColorConfig, OrthographicProjection, ScalingMode};
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;

mod conf;
mod game;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, (spawn_ui_camera, spawn_game_camera))
        .add_plugins(TiledPlugin::default())
        .add_plugins((game::GamePlugin, ui::UiPlugin))
        .run();
}

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        RenderLayers::layer(1),
        IsDefaultUiCamera,
    ));
}

fn spawn_game_camera(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: conf::GAME_VIEW_WIDTH,
        height: conf::GAME_VIEW_HEIGHT,
    };
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Projection::Orthographic(projection),
        ui::gameview::GameCamera,
        Transform::default(),
        GlobalTransform::default(),
        Name::new("Game Camera"),
    ));
}
