use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

mod chat;
mod game_overlay;
mod leftpanel;
mod rightpanel;
mod toppanel;
mod window;

pub use window::AddUIWindow;

#[derive(Resource)]
pub struct UiFonts {
    // pub main_font: Handle<Font>,
    pub content_font: Handle<Font>,
}

#[derive(Component)]
pub struct MainUI;

// #[derive(Resource)]
// pub struct UiAssets {
//     pub resize_cursor: Handle<Image>,
// }

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(window::UIWindowPlugin)
            .add_systems(Startup, spawn_main_ui)
            .add_systems(PostUpdate, game_overlay::set_game_camera_to_viewport)
            .add_systems(
                Update,
                (
                    toppanel::update_ui_bars_fill,
                    toppanel::update_health_fill_color,
                ),
            )
            .add_systems(
                Update,
                (
                    toppanel::update_health,
                    toppanel::update_mana,
                    toppanel::update_experience,
                ),
            );
    }
}

// fn load_ui_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
// commands.insert_resource(UiAssets {
//     resize_cursor: asset_server.load("ui/resize.png"),
// });
// }

fn spawn_main_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let fonts = UiFonts {
        // main_font: asset_server.load("fonts/Aldrich-Regular.ttf"),
        content_font: asset_server.load("fonts/RubikMonoOne-Regular.ttf"),
    };

    let main_ui = commands
        .spawn((
            MainUI,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            RenderLayers::layer(1),
        ))
        .id();

    let left_panel = leftpanel::spawn_left_panel(&mut commands, &asset_server);
    let middle_container = commands
        .spawn((Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        },))
        .id();
    let right_panel = rightpanel::spawn_right_panel(&mut commands, &asset_server);
    commands
        .entity(main_ui)
        .add_children(&[left_panel, middle_container, right_panel]);

    let top_panel = toppanel::spawn_top_panel(&mut commands, &asset_server, &fonts);
    let gameview = game_overlay::spawn_gameviewport(&mut commands);
    let chat = chat::spawn_chat(&mut commands, &asset_server);
    commands
        .entity(middle_container)
        .add_children(&[top_panel, gameview, chat]);

    commands.insert_resource(fonts);
}
