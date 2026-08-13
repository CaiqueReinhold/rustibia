use bevy::prelude::*;

use crate::game_ui::GameUiAssets;
use crate::game_ui::modal::{DialogButtonPressed, ModalDialog, ModalOrder};
use crate::network::events::ClientOutdated;

#[derive(Component)]
pub(super) struct ClientOutdatedModal;

pub(super) fn on_client_outdated(
    _: On<ClientOutdated>,
    mut commands: Commands,
    ui_assets: Res<GameUiAssets>,
    mut order: ResMut<ModalOrder>,
    existing: Query<(), With<ClientOutdatedModal>>,
) {
    if !existing.is_empty() {
        return;
    }
    let root = ModalDialog::message(
        "Client Outdated",
        "Your client is outdated. Please update to the newest version.",
        &mut commands,
        &ui_assets,
        &mut order,
    );
    commands.entity(root).insert(ClientOutdatedModal);
}

pub(super) fn on_dismiss(
    event: On<DialogButtonPressed>,
    modals: Query<(), With<ClientOutdatedModal>>,
    mut commands: Commands,
) {
    if modals.contains(event.dialog) {
        commands.write_message(AppExit::Success);
    }
}
