use std::time::Duration;

use bevy::prelude::*;
use bevy::{camera::visibility::RenderLayers, time::common_conditions::on_timer};

mod assets;
mod chat;
mod game_overlay;
mod leftpanel;
mod rightpanel;
mod toppanel;
mod window;

pub use assets::GameUiAssets;
pub use game_overlay::GameViewport;
pub use rightpanel::RightPanelDock;
pub use window::{
    AddUIWindow, CloseUIWindow, Index, ReplaceUIWindowContent, UIWindow, UIWindowDock, UiWindowRef,
    WindowId,
};

use crate::camera::GameRenderTexture;
use crate::core::{GameState, PingState};

#[derive(Component)]
pub struct MainUI;

#[derive(Component)]
pub struct PingView;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(window::UIWindowPlugin)
            .add_systems(OnEnter(GameState::InGame), spawn_main_ui)
            .add_systems(Startup, assets::setup_game_ui_assets)
            .add_systems(
                Update,
                toppanel::update_bar.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                game_overlay::update_viewport_size.run_if(in_state(GameState::InGame)),
            )
            .add_systems(Update, update_ping.run_if(on_timer(Duration::from_secs(1))));
    }
}

// fn load_ui_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
// commands.insert_resource(UiAssets {
//     resize_cursor: asset_server.load("ui/resize.png"),
// });
// }

pub(crate) fn spawn_main_ui(
    mut commands: Commands,
    render_texture: Res<GameRenderTexture>,
    ui_assets: Res<GameUiAssets>,
) {
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

    let left_panel = leftpanel::spawn_left_panel(&mut commands, &ui_assets);
    let middle_container = commands
        .spawn((Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        },))
        .id();
    let right_panel = rightpanel::spawn_right_panel(&mut commands, &ui_assets);
    commands
        .entity(main_ui)
        .add_children(&[left_panel, middle_container, right_panel]);

    let top_panel = toppanel::spawn_top_panel(&mut commands, &ui_assets);
    let gameview = game_overlay::spawn_gameviewport(&mut commands, &render_texture, &ui_assets);
    let chat = chat::spawn_chat(&mut commands, &ui_assets);
    commands
        .entity(middle_container)
        .add_children(&[top_panel, gameview, chat]);

    let ping_view = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(15.0),
                ..default()
            },
            Children::spawn(Spawn((
                PingView,
                Text::new(""),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ))),
        ))
        .id();
    commands.entity(gameview).add_child(ping_view);
}

pub fn update_ping(mut ping_text: Single<&mut Text, With<PingView>>, ping_state: Res<PingState>) {
    let text = format!("Ping: {}ms", ping_state.current_ping.as_millis());
    ping_text.0 = text;
}
