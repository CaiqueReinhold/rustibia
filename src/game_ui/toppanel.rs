use bevy::prelude::*;

use crate::actor::{Health, HealthState, HudBar, Mana};
use crate::conf::ui::{ui_colors, TOP_BAR_HEIGHT, UI_BAR_HEIGHT};
use crate::game_ui::assets::GameUiAssets;
use crate::player::components::Player;

#[derive(Component)]
pub struct TopPanel;

#[derive(Resource)]
pub struct BarEntities {
    pub health: Entity,
    pub mana: Entity,
    // pub experience: Entity,
}

pub fn spawn_top_panel(commands: &mut Commands, ui_assets: &GameUiAssets) -> Entity {
    let top_panel = commands
        .spawn((
            TopPanel,
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                max_height: Val::Px(TOP_BAR_HEIGHT),
                min_height: Val::Px(TOP_BAR_HEIGHT),
                border: UiRect {
                    top: Val::Px(1.0),
                    left: Val::Px(2.0),
                    right: Val::Px(2.0),
                    bottom: Val::Px(2.0),
                },
                ..default()
            },
            BorderColor {
                top: ui_colors::LIGHT_BORDER_COLOR.into(),
                right: ui_colors::DARK_BORDER_COLOR.into(),
                bottom: ui_colors::DARK_BORDER_COLOR.into(),
                left: ui_colors::LIGHT_BORDER_COLOR.into(),
            },
            ZIndex(5),
        ))
        .id();

    let panel_inner = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            ImageNode {
                image: ui_assets.background_light.clone(),
                image_mode: NodeImageMode::Tiled {
                    tile_x: true,
                    tile_y: true,
                    stretch_value: 1.0,
                },
                ..default()
            },
        ))
        .id();

    commands.entity(top_panel).add_child(panel_inner);

    let bars_container = commands
        .spawn((Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            row_gap: Val::Px(3.0),
            ..default()
        },))
        .id();
    commands.entity(panel_inner).add_child(bars_container);

    let health = spawn_ui_bar(bars_container, commands, true, ui_assets);
    let mana = spawn_ui_bar(bars_container, commands, false, ui_assets);

    commands.insert_resource(BarEntities {
        health,
        mana,
        // experience,
    });

    top_panel
}

fn spawn_ui_bar(
    parent: Entity,
    commands: &mut Commands,
    add_health_state: bool,
    ui_assets: &GameUiAssets,
) -> Entity {
    let hud_bar = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            HudBar { ratio: 1.0 },
            ImageNode {
                image: ui_assets.bar_overlay.clone(),
                image_mode: NodeImageMode::Tiled {
                    tile_x: true,
                    tile_y: false,
                    stretch_value: 1.0,
                },
                ..default()
            },
        ))
        .id();
    if add_health_state {
        commands.entity(hud_bar).insert(HealthState::Full);
    } else {
        commands
            .entity(hud_bar)
            .insert(BackgroundColor(ui_colors::MANA_BAR_COLOR.into()));
    }

    let bar = commands
        .spawn((
            Node {
                width: Val::Percent(50.0),
                height: Val::Px(UI_BAR_HEIGHT),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor {
                top: ui_colors::DARK_BORDER_COLOR.into(),
                right: ui_colors::LIGHT_BORDER_COLOR.into(),
                bottom: ui_colors::LIGHT_BORDER_COLOR.into(),
                left: ui_colors::DARK_BORDER_COLOR.into(),
            },
        ))
        .add_child(hud_bar)
        .id();
    commands.entity(parent).add_child(bar);

    hud_bar
}

pub fn update_bar_ratio(
    bars: Res<BarEntities>,
    player: Single<(&Health, &Mana), (With<Player>, Or<(Changed<Health>, Changed<Mana>)>)>,
    mut bar_q: Query<&mut HudBar>,
) {
    let (health, mana) = *player;

    if let Ok(mut health_bar) = bar_q.get_mut(bars.health) {
        health_bar.ratio = health.ratio();
    }

    if let Ok(mut mana_bar) = bar_q.get_mut(bars.mana) {
        mana_bar.ratio = mana.ratio();
    }
}
