use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_text_outline::TextOutline;

use crate::conf::ui::login as conf;
use crate::core::GameState;
use crate::game_ui::GameUiAssets;

mod charlist;
mod form;

/// Which login dialog is showing. Exists only while
/// `GameState::LoginScreen` is active; resets to `EnterGame` on re-entry.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::LoginScreen)]
pub enum LoginPhase {
    #[default]
    EnterGame,
    CharacterList,
}

/// Error dialog (title, message) carried across the Connecting →
/// LoginScreen transition so the modal spawns AFTER the login form
/// (and therefore stacks on top).
#[derive(Resource, Default)]
pub struct PendingLoginError(pub Option<(String, String)>);

#[derive(Component)]
struct LoginBackdrop;

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<LoginPhase>()
            .init_resource::<PendingLoginError>()
            .add_systems(OnEnter(GameState::LoginScreen), spawn_backdrop)
            .add_systems(OnExit(GameState::LoginScreen), despawn_backdrop)
            .add_systems(
                OnEnter(LoginPhase::EnterGame),
                (form::spawn_login_form, form::show_pending_error).chain(),
            )
            .add_systems(OnExit(LoginPhase::EnterGame), form::despawn_login_form)
            .add_systems(
                Update,
                (
                    form::tab_switch_field,
                    form::on_login_field_submit,
                    form::update_password_display,
                )
                    .run_if(in_state(LoginPhase::EnterGame)),
            )
            .add_observer(form::on_login_dialog_button)
            .add_observer(form::on_attempt_login)
            .add_systems(
                OnEnter(LoginPhase::CharacterList),
                charlist::spawn_character_list,
            )
            .add_systems(
                OnExit(LoginPhase::CharacterList),
                charlist::despawn_character_list,
            )
            .add_systems(
                Update,
                (
                    charlist::keyboard_navigation,
                    charlist::update_row_highlight,
                )
                    .chain()
                    .run_if(in_state(LoginPhase::CharacterList)),
            )
            .add_observer(charlist::on_charlist_dialog_button)
            .add_observer(charlist::on_confirm_character)
            .add_observer(on_login_error)
            .add_observer(on_connection_lost);
    }
}

fn on_login_error(
    _: On<crate::network::events::LoginError>,
    state: Res<State<GameState>>,
    mut pending: ResMut<PendingLoginError>,
    mut commands: Commands,
) {
    if *state.get() != GameState::Connecting {
        return;
    }
    pending.0 = Some((
        "Login Failed".to_string(),
        "Your character could not be logged in. Please try again.".to_string(),
    ));
    commands.set_state(GameState::LoginScreen);
}

fn on_connection_lost(
    _: On<crate::network::events::ConnectionLost>,
    state: Res<State<GameState>>,
    mut pending: ResMut<PendingLoginError>,
    mut commands: Commands,
) {
    if *state.get() != GameState::Connecting {
        return;
    }
    pending.0 = Some((
        "Connection Error".to_string(),
        "Cannot connect to the game server.".to_string(),
    ));
    commands.set_state(GameState::LoginScreen);
}

fn spawn_backdrop(mut commands: Commands, ui_assets: Res<GameUiAssets>) {
    commands
        .spawn((
            LoginBackdrop,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            // Stretch-fills the window; non-16:9 ratios distort the art.
            // Bevy 0.18 has no cover/contain image mode — don't "fix" this
            // by switching to Stretch (identical) or chasing a mode that
            // doesn't exist.
            ImageNode {
                image: ui_assets.title_background.clone(),
                ..default()
            },
            RenderLayers::layer(1),
        ))
        .with_child((
            Text::new("RUSTIBIA"),
            TextFont {
                font: ui_assets.font.clone(),
                font_size: conf::LOGO_FONT_SIZE,
                ..default()
            },
            TextColor(conf::LOGO_COLOR.into()),
            TextOutline {
                width: 3.0,
                color: Color::BLACK,
            },
            Node {
                margin: UiRect::top(Val::Px(conf::LOGO_TOP_MARGIN)),
                ..default()
            },
        ));
}

fn despawn_backdrop(mut commands: Commands, backdrops: Query<Entity, With<LoginBackdrop>>) {
    for entity in &backdrops {
        commands.entity(entity).despawn();
    }
}
