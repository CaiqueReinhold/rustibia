use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    conf::{
        minimap::{DEFAULT_ZOOM, IMAGE_SIZE, ZOOM_LEVELS},
        ui::{ui_colors, z_index::Z_WINDOW, SIDE_PANEL_WIDTH},
    },
    core::GameState,
    game_ui::{GameUiAssets, Index, RightPanelDock, UIWindow, UIWindowDock, UiWindowRef, WindowId},
    map::{minimap::MinimapData, Position},
    player::components::Player,
};

#[derive(Resource)]
pub struct MinimapImageHandle(pub Handle<Image>);

#[derive(Resource)]
pub struct MinimapZoom(pub usize);

#[derive(Component)]
struct MinimapImageNode;

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::InGame),
            setup_minimap
                .after(crate::game_ui::spawn_main_ui)
                .before(crate::items::inventory::spawn_inventory_ui),
        )
        .add_systems(
            Update,
            update_minimap_image.run_if(in_state(GameState::InGame)),
        );
    }
}

fn setup_minimap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    dock_q: Query<(Entity, &UIWindowDock), With<RightPanelDock>>,
    ui_assets: Res<GameUiAssets>,
) {
    let Ok((dock_entity, dock)) = dock_q.single() else {
        return;
    };

    let mut image = Image::new_fill(
        Extent3d {
            width: IMAGE_SIZE,
            height: IMAGE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    let handle = images.add(image);

    commands.insert_resource(MinimapImageHandle(handle.clone()));
    commands.insert_resource(MinimapZoom(DEFAULT_ZOOM));

    let window_id = WindowId::new();
    let zoom_tiles = ZOOM_LEVELS[DEFAULT_ZOOM] as f32;
    // Interior width = SIDE_PANEL_WIDTH minus 2px border on each side
    let window_height = 120.0 + 20.0 + 4.0; // image + button row + border

    // Zoom-in button ("+")
    let zoom_in_btn = commands
        .spawn((
            Node {
                width: Val::Px(20.0),
                height: Val::Px(16.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor {
                top: ui_colors::LIGHT_BORDER_COLOR.into(),
                left: ui_colors::LIGHT_BORDER_COLOR.into(),
                bottom: ui_colors::DARK_BORDER_COLOR.into(),
                right: ui_colors::DARK_BORDER_COLOR.into(),
            },
        ))
        .with_child((
            Text::new("+"),
            TextFont {
                font: ui_assets.font.clone(),
                font_size: 10.0,
                ..default()
            },
            TextColor(Color::from(ui_colors::FONT_COLOR_CONTENT)),
        ))
        .observe(|mut e: On<Pointer<Click>>, mut zoom: ResMut<MinimapZoom>| {
            e.propagate(false);
            if zoom.0 > 0 {
                zoom.0 -= 1;
            }
        })
        .id();

    // Zoom-out button ("−")
    let zoom_out_btn = commands
        .spawn((
            Node {
                width: Val::Px(20.0),
                height: Val::Px(16.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor {
                top: ui_colors::LIGHT_BORDER_COLOR.into(),
                left: ui_colors::LIGHT_BORDER_COLOR.into(),
                bottom: ui_colors::DARK_BORDER_COLOR.into(),
                right: ui_colors::DARK_BORDER_COLOR.into(),
            },
        ))
        .with_child((
            Text::new("−"),
            TextFont {
                font: ui_assets.font.clone(),
                font_size: 10.0,
                ..default()
            },
            TextColor(Color::from(ui_colors::FONT_COLOR_CONTENT)),
        ))
        .observe(|mut e: On<Pointer<Click>>, mut zoom: ResMut<MinimapZoom>| {
            e.propagate(false);
            if zoom.0 < ZOOM_LEVELS.len() - 1 {
                zoom.0 += 1;
            }
        })
        .id();

    // Button row
    let button_row = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(20.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            column_gap: Val::Px(2.0),
            padding: UiRect::horizontal(Val::Px(2.0)),
            ..default()
        })
        .add_children(&[zoom_in_btn, zoom_out_btn])
        .id();

    // White cross marker — always centered (player is always at rect center)
    let cross_h = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(7.0),
                height: Val::Px(1.0),
                left: Val::Px(120.0 / 2.0 - 3.0),
                top: Val::Px(120.0 / 2.0),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .id();

    let cross_v = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(1.0),
                height: Val::Px(7.0),
                left: Val::Px(120.0 / 2.0),
                top: Val::Px(120.0 / 2.0 - 3.0),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .id();

    // Minimap image node
    let image_node = commands
        .spawn((
            MinimapImageNode,
            Node {
                width: Val::Px(120.0),
                height: Val::Px(120.0),
                overflow: Overflow::visible(),
                ..default()
            },
            ImageNode {
                image: handle,
                rect: Some(Rect {
                    min: Vec2::ZERO,
                    max: Vec2::new(zoom_tiles, zoom_tiles),
                }),
                ..default()
            },
        ))
        .add_children(&[cross_h, cross_v])
        .id();

    // Content container
    let content = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_items: JustifyItems::Center,
            ..default()
        })
        .add_children(&[image_node, button_row])
        .id();

    commands.entity(content).insert(UiWindowRef { window_id });

    let window = commands
        .spawn((
            UIWindow {
                id: window_id,
                dock_id: dock.id,
            },
            Index(0),
            Node {
                left: Val::Px(-2.0),
                width: Val::Px(SIDE_PANEL_WIDTH),
                height: Val::Px(window_height),
                min_height: Val::Px(window_height),
                border: UiRect::all(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::hidden(),
                ..default()
            },
            BorderColor {
                top: ui_colors::LIGHT_BORDER_COLOR.into(),
                left: ui_colors::LIGHT_BORDER_COLOR.into(),
                bottom: ui_colors::DARK_BORDER_COLOR.into(),
                right: ui_colors::DARK_BORDER_COLOR.into(),
            },
            ZIndex(Z_WINDOW),
        ))
        .add_child(content)
        .id();

    commands.entity(dock_entity).add_child(window);
}

fn update_minimap_image(
    mut minimap: ResMut<MinimapData>,
    image_handle: Option<Res<MinimapImageHandle>>,
    mut images: ResMut<Assets<Image>>,
    zoom: Option<Res<MinimapZoom>>,
    player_q: Single<&Position, With<Player>>,
    mut image_node_q: Query<&mut ImageNode, With<MinimapImageNode>>,
    mut prev_floor: Local<Option<u32>>,
) {
    let Some(handle) = image_handle else {
        return;
    };
    let Some(zoom) = zoom else {
        return;
    };

    let player = player_q.into_inner();
    let current_z = player.z;

    // Update ImageNode rect to keep the player centered
    if let Ok(mut img) = image_node_q.single_mut() {
        img.rect = Some(player_centered_rect(player, &zoom));
    }

    // On floor change, repaint all chunks for the new floor
    if prev_floor.is_none_or(|z| z != current_z) {
        minimap.mark_floor_gpu_dirty(current_z);
        *prev_floor = Some(current_z);
    }

    let dirty = minimap.drain_gpu_dirty(current_z);
    if dirty.is_empty() {
        return;
    }

    let Some(image) = images.get_mut(&handle.0) else {
        return;
    };
    let Some(ref mut data) = image.data else {
        return;
    };

    for (key, tiles) in dirty {
        for ly in 0..64u32 {
            for lx in 0..64u32 {
                let tile = tiles[(ly * 64 + lx) as usize];
                let px = key.cx * 64 + lx;
                let py = key.cy * 64 + ly;
                if px >= IMAGE_SIZE || py >= IMAGE_SIZE {
                    continue;
                }
                let idx = (py * IMAGE_SIZE + px) as usize * 4;
                let (r, g, b) = minimap_color_to_rgb(tile.color);
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = 255;
            }
        }
    }
}

fn player_centered_rect(player: &Position, zoom: &MinimapZoom) -> Rect {
    let tiles = ZOOM_LEVELS[zoom.0] as f32;
    let half = tiles / 2.0;
    let cx = player.x as f32;
    let cy = player.y as f32;
    // Clamp so the rect never goes outside the image while keeping its size constant
    let max_origin = (IMAGE_SIZE as f32 - tiles).max(0.0);
    let min_x = (cx - half).max(0.0).min(max_origin);
    let min_y = (cy - half).max(0.0).min(max_origin);
    Rect {
        min: Vec2::new(min_x, min_y),
        max: Vec2::new(min_x + tiles, min_y + tiles),
    }
}

fn minimap_color_to_rgb(color: u8) -> (u8, u8, u8) {
    (color / 36 % 6 * 51, color / 6 % 6 * 51, color % 6 * 51)
}
