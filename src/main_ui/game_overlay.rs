use bevy::camera::{Camera, Viewport};
use bevy::prelude::*;

use crate::camera::GameCamera;
use crate::conf::viewport::{ASPECT_RATIO, GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH};

#[derive(Resource, Debug, Default)]
pub struct GameScaleFactor(pub f32);

#[derive(Component)]
pub struct GameViewport;

pub fn spawn_gameviewport(commands: &mut Commands) -> Entity {
    let viewport = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GameViewport,
        ))
        .id();

    viewport
}

pub fn set_game_camera_to_viewport(
    windows: Query<&Window>,
    game_node: Query<(&ComputedNode, &UiGlobalTransform), With<GameViewport>>,
    mut camera: Query<&mut Camera, With<GameCamera>>,
    mut game_scale: ResMut<GameScaleFactor>,
) {
    let Ok((node, transform)) = game_node.single() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut camera) = camera.single_mut() else {
        return;
    };
    let size = node.size();

    let scale_factor = window.resolution.scale_factor();

    let physical_width_ratio = size.y * ASPECT_RATIO;
    let physical_height_ratio = size.x / ASPECT_RATIO;

    let physical_width;
    let physical_height;
    let physical_x;
    let physical_y;

    if size.x / size.y >= ASPECT_RATIO {
        physical_width = (physical_width_ratio * scale_factor).round();
        physical_height = (size.y * scale_factor).round();
        physical_x = (transform.translation.x - physical_width / 2.0).round();
        physical_y = (transform.translation.y - size.y / 2.0).round();
    } else {
        physical_width = (size.x * scale_factor).round();
        physical_height = (physical_height_ratio * scale_factor).round();
        physical_x = (transform.translation.x - size.x / 2.0).round();
        physical_y = (transform.translation.y - physical_height / 2.0).round();
    }

    if physical_width == 0.0 || physical_height == 0.0 {
        return;
    }

    game_scale.0 = f32::min(
        physical_height / (GAME_VIEW_HEIGHT * scale_factor),
        physical_width / (GAME_VIEW_WIDTH * scale_factor),
    );

    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(physical_x as u32, physical_y as u32),
        physical_size: UVec2::new(physical_width as u32, physical_height as u32),
        ..default()
    });
}
