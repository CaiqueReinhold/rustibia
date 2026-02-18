use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::main_ui::window::UIWindowDock;

mod chat;
mod gameview;
mod leftpanel;
mod rightpanel;
mod toppanel;
mod window;

#[derive(Resource)]
pub struct UiFonts {
    // pub main_font: Handle<Font>,
    pub content_font: Handle<Font>,
}

// #[derive(Resource)]
// pub struct UiAssets {
//     pub resize_cursor: Handle<Image>,
// }

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(window::UIWindowPlugin)
            .add_systems(Startup, spawn_main_ui)
            .add_systems(PostUpdate, gameview::set_game_camera_to_viewport)
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
            )
            .add_observer(trigger_window);
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
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            RenderLayers::layer(1),
            Name::new("Main UI"),
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
    let gameview = gameview::spawn_gameviewport(&mut commands);
    let chat = chat::spawn_chat(&mut commands, &asset_server);
    commands
        .entity(middle_container)
        .add_children(&[top_panel, gameview, chat]);

    commands.insert_resource(fonts);
}

fn trigger_window(_: On<Add, UIWindowDock>, mut commands: Commands) {
    let entity = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(180.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Children::spawn((
                Spawn((Text::new("1"), TextColor(Color::WHITE))),
                Spawn((Text::new("2"), TextColor(Color::WHITE))),
                Spawn((Text::new("3"), TextColor(Color::WHITE))),
                Spawn((Text::new("4"), TextColor(Color::WHITE))),
                Spawn((Text::new("5"), TextColor(Color::WHITE))),
                Spawn((Text::new("6"), TextColor(Color::WHITE))),
                Spawn((Text::new("7"), TextColor(Color::WHITE))),
                Spawn((Text::new("8"), TextColor(Color::WHITE))),
                Spawn((Text::new("9"), TextColor(Color::WHITE))),
                Spawn((Text::new("10"), TextColor(Color::WHITE))),
                (
                    Spawn((Text::new("11"), TextColor(Color::WHITE))),
                    Spawn((Text::new("12"), TextColor(Color::WHITE))),
                    Spawn((Text::new("13"), TextColor(Color::WHITE))),
                    Spawn((Text::new("14"), TextColor(Color::WHITE))),
                    Spawn((Text::new("15"), TextColor(Color::WHITE))),
                    Spawn((Text::new("16"), TextColor(Color::WHITE))),
                ),
            )),
        ))
        .id();
    let entity2 = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(200.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Children::spawn(Spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(150.0),
                    ..default()
                },
                BackgroundColor(Srgba::new(0.224, 0.224, 0.243, 1.0).into()),
            ))),
        ))
        .id();
    let entity3 = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(180.0),
                ..default()
            },
            Outline {
                width: Val::Px(5.0),
                offset: Val::ZERO,
                color: Color::WHITE,
            }, // BackgroundColor(Color::BLACK),
        ))
        .id();
    commands.trigger(window::AddUIWindow {
        content: entity,
        default_height: 100,
        title: "test window 1".to_string(),
    });
    commands.trigger(window::AddUIWindow {
        content: entity2,
        default_height: 100,
        title: "test window 2".to_string(),
    });
    commands.trigger(window::AddUIWindow {
        content: entity3,
        default_height: 100,
        title: "test window 3".to_string(),
    });
}
