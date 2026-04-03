use bevy::prelude::*;

pub struct UiFonts {
    pub main_font: Handle<Font>,
    pub content_font: Handle<Font>,
}

pub struct UiWindow {
    pub close_button: Handle<Image>,
    pub parent_container: Handle<Image>,
    pub minimize_button: Handle<Image>,
}

#[derive(Resource)]
pub struct GameUiAssets {
    pub fonts: UiFonts,
    pub window: UiWindow,
}

pub fn setup_game_ui_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let fonts = UiFonts {
        main_font: asset_server.load("fonts/Verdana.ttf"),
        content_font: asset_server.load("fonts/RubikMonoOne-Regular.ttf"),
    };

    let window = UiWindow {
        close_button: asset_server.load("ui/window_close.png"),
        parent_container: asset_server.load("ui/window_parent.png"),
        minimize_button: asset_server.load("ui/window_minimize.png"),
    };

    commands.insert_resource(GameUiAssets { fonts, window });
}
