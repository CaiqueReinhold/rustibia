use bevy::camera::{Camera, Viewport};
use bevy::prelude::*;

use crate::camera::GameCamera;
use crate::conf::ui::{SIDE_PANEL_WIDTH, TOP_BAR_HEIGHT};
use crate::conf::viewport::ASPECT_RATIO;
use crate::map::TilePosition;

#[derive(Component)]
pub struct GameViewport;

// #[derive(Component)]
// pub struct GameFrame;

fn on_drag(
    event: On<Pointer<DragStart>>,
    camera: Single<(&Camera, &GlobalTransform), With<GameCamera>>,
) {
    let (camera, camera_transform) = *camera;
    let click_postion = event.pointer_location.position;
    let Some(viewport) = &camera.viewport else {
        return;
    };
    let viewport_rect = Rect::from_corners(
        viewport.physical_position.as_vec2(),
        (viewport.physical_position + viewport.physical_size).as_vec2(),
    );

    if viewport_rect.contains(click_postion) {
        let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, click_postion) else {
            return;
        };

        info!(
            "world pos: {:?} tile pos: {:?}",
            world_pos,
            TilePosition::from_world(world_pos, 7)
        );
    }
}

pub fn spawn_gameviewport(commands: &mut Commands) -> Entity {
    // let bg_box = asset_server.load("ui/frame.png");

    // let slicer = TextureSlicer {
    //     border: BorderRect::all(100.0),
    //     center_scale_mode: SliceScaleMode::Stretch,
    //     sides_scale_mode: SliceScaleMode::Stretch,
    //     max_corner_scale: 1.0,
    // };

    let viewport = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GameViewport,
            Name::new("Game Viewport"),
        ))
        .observe(on_drag)
        .id();

    // let frame = commands
    //     .spawn((
    //         GameFrame,
    //         Node {
    //             position_type: PositionType::Absolute,
    //             top: Val::Px(0.0),
    //             left: Val::Px(0.0),
    //             width: Val::Percent(100.0),
    //             height: Val::Percent(100.0),
    //             ..default()
    //         },
    //         (ImageNode {
    //             image: bg_box,
    //             ..default()
    //         })
    //         .with_mode(NodeImageMode::Sliced(slicer)),
    //     ))
    //     .id();
    // commands.entity(viewport).add_child(frame);

    viewport
}

pub fn set_game_camera_to_viewport(
    windows: Query<&Window>,
    // mut frame_node: Query<&mut UiTransform, With<GameFrame>>,
    game_node: Query<&ComputedNode, With<GameViewport>>,
    mut camera: Query<&mut Camera, With<GameCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    // let mut frame_node = frame_node.single_mut().unwrap();
    let node = game_node.single().unwrap();
    let mut camera = camera.single_mut().unwrap();
    let size = node.size();

    let scale_factor = window.resolution.scale_factor();

    let physical_width_ratio = size.y * ASPECT_RATIO;
    let physical_height_ratio = size.x / ASPECT_RATIO;

    let physical_width;
    let physical_height;
    let physical_x;
    let physical_y;

    if size.x / size.y >= ASPECT_RATIO {
        physical_width = (physical_width_ratio * scale_factor).round() as u32;
        physical_height = (size.y * scale_factor).round() as u32;
        physical_x = (((size.x - physical_width_ratio) / 2.0).round() + SIDE_PANEL_WIDTH) as u32;
        physical_y = TOP_BAR_HEIGHT as u32 + 1;
    } else {
        physical_width = (size.x * scale_factor).round() as u32;
        physical_height = (physical_height_ratio * scale_factor).round() as u32;
        physical_x = SIDE_PANEL_WIDTH as u32;
        physical_y = (((size.y - physical_height_ratio) / 2.0).round() as u32) + 100;
    }

    if physical_width == 0 || physical_height == 0 {
        return;
    }

    // frame_node.translation = Val2::px(
    //     (physical_width_ratio - (physical_width as f32) + 2.0) / 2.0,
    //     (physical_y + 2) as f32,
    // );
    // frame_node.scale.x = (physical_width + 8) as f32 / size.x;
    // frame_node.scale.y = (physical_height + 8) as f32 / size.y;

    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(physical_x, physical_y),
        physical_size: UVec2::new(physical_width, physical_height),
        ..default()
    });
}
