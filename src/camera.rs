use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Camera, ClearColorConfig, OrthographicProjection, ScalingMode};
use bevy::prelude::*;

use crate::conf::viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH};

#[derive(Component)]
pub struct GameCamera;

pub fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            // msaa_writeback: MsaaWriteback::Off,
            ..default()
        },
        RenderLayers::layer(1),
        Msaa::Off,
        IsDefaultUiCamera,
    ));
}

pub fn spawn_game_camera(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: GAME_VIEW_WIDTH,
        height: GAME_VIEW_HEIGHT,
    };
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        Projection::Orthographic(projection),
        GameCamera,
        Transform::default(),
        GlobalTransform::default(),
        Msaa::Off,
    ));
}
