use bevy::prelude::*;

use crate::actor::{Health, HealthState, Mana};
use crate::conf::ui::TOP_BAR_HEIGHT;
use crate::main_ui::UiFonts;
use crate::player::components::Player;

#[derive(Component)]
pub struct TopPanel;

#[derive(Component)]
pub struct BarUI {
    max: u32,
    current: u32,
    health_state: Option<HealthState>,
    show_text: bool,
}

#[derive(Resource)]
pub struct BarAssets {
    pub background: Handle<Image>,
    pub fill_dark_red: Handle<Image>,
    pub fill_red: Handle<Image>,
    pub fill_yellow: Handle<Image>,
    pub fill_light_green: Handle<Image>,
    pub fill_dark_green: Handle<Image>,
    pub fill_blue: Handle<Image>,
    pub fill_small_green: Handle<Image>,
}

#[derive(Resource)]
pub struct BarEntities {
    pub health: Entity,
    pub mana: Entity,
    pub experience: Entity,
}

pub fn spawn_top_panel(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    fonts: &UiFonts,
) -> Entity {
    let bar_assets = BarAssets {
        background: asset_server.load("ui/bar_background.png"),
        fill_dark_red: asset_server.load("ui/bar_dark_red.png"),
        fill_red: asset_server.load("ui/bar_red.png"),
        fill_yellow: asset_server.load("ui/bar_yellow.png"),
        fill_light_green: asset_server.load("ui/bar_light_green.png"),
        fill_dark_green: asset_server.load("ui/bar_dark_green.png"),
        fill_blue: asset_server.load("ui/bar_blue.png"),
        fill_small_green: asset_server.load("ui/small_fill_green.png"),
    };
    let bg_box = asset_server.load("ui/box_under.png");

    let slicer = TextureSlicer {
        border: BorderRect::all(50.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };

    let top_panel = commands
        .spawn((
            TopPanel,
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                max_height: Val::Px(TOP_BAR_HEIGHT - 2.0),
                min_height: Val::Px(TOP_BAR_HEIGHT - 2.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Start,
                align_content: AlignContent::Start,
                row_gap: Val::Px(5.0),
                padding: UiRect::all(Val::Px(4.0)),
                flex_wrap: FlexWrap::Wrap,
                margin: UiRect {
                    bottom: Val::Px(2.0),
                    ..default()
                },
                ..default()
            },
            (ImageNode {
                image: bg_box,
                ..default()
            })
            .with_mode(NodeImageMode::Sliced(slicer)),
            ZIndex(5),
        ))
        .id();

    let health = spawn_ui_bar(
        commands,
        50.0,
        24.0,
        5.0,
        true,
        bar_assets.background.clone(),
        bar_assets.fill_dark_green.clone(),
        fonts.content_font.clone(),
    );
    let mana = spawn_ui_bar(
        commands,
        50.0,
        24.0,
        5.0,
        true,
        bar_assets.background.clone(),
        bar_assets.fill_blue.clone(),
        fonts.content_font.clone(),
    );
    let experience = spawn_ui_bar(
        commands,
        100.0,
        8.0,
        2.0,
        false,
        bar_assets.background.clone(),
        bar_assets.fill_small_green.clone(),
        fonts.content_font.clone(),
    );

    commands
        .entity(top_panel)
        .add_children(&[health, mana, experience]);

    commands.insert_resource(bar_assets);
    commands.insert_resource(BarEntities {
        health,
        mana,
        experience,
    });

    top_panel
}

fn spawn_ui_bar(
    commands: &mut Commands,
    width: f32,
    height: f32,
    pad: f32,
    show_text: bool,
    background: Handle<Image>,
    fill: Handle<Image>,
    font: Handle<Font>,
) -> Entity {
    let slicer = TextureSlicer {
        border: BorderRect::all(40.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(width),
                height: Val::Px(height),
                flex_direction: FlexDirection::Row,
                align_content: AlignContent::Start,
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(Val::Px(pad)),
                ..default()
            },
            (ImageNode {
                image: background,
                ..default()
            })
            .with_mode(NodeImageMode::Sliced(slicer.clone())),
            BarUI {
                max: 1,
                current: 0,
                health_state: None,
                show_text,
            },
            ZIndex(6),
        ))
        .with_child((
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                height: Val::Px(height - (pad * 2.0)),
                ..default()
            },
            (ImageNode {
                image: fill,
                ..default()
            })
            .with_mode(NodeImageMode::Sliced(slicer)),
            ZIndex(7),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ZIndex(8),
                ))
                .with_child((
                    Text::new(""),
                    TextLayout::new(Justify::Center, LineBreak::NoWrap),
                    TextFont {
                        font,
                        font_size: 8.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
        })
        .id()
}

pub fn update_ui_bars_fill(
    changed_bars: Query<(&BarUI, &Children), Changed<BarUI>>,
    mut node_query: Query<(&mut Node, Option<&Children>)>,
    mut text_query: Query<&mut Text>,
) {
    for (bar, children) in changed_bars.iter() {
        let Some(node_child) = children.first() else {
            continue;
        };
        let (mut fill_node, _) = node_query.get_mut(*node_child).unwrap();
        fill_node.width = Val::Percent(bar.current as f32 / bar.max as f32 * 100.0);

        if bar.show_text {
            let Some(node_child2) = children.get(1) else {
                continue;
            };
            let (_, text_children_opt) = node_query.get_mut(*node_child2).unwrap();
            let Some(text_children) = text_children_opt else {
                continue;
            };
            let Some(text_entity) = text_children.first() else {
                continue;
            };
            let mut text = text_query.get_mut(*text_entity).unwrap();
            text.0 = format!("{}/{}", bar.current, bar.max);
        }
    }
}

pub fn update_health_fill_color(
    changed_bars: Query<(&BarUI, &Children), Changed<BarUI>>,
    mut node_query: Query<&mut ImageNode>,
    bar_assets: Res<BarAssets>,
) {
    for (bar, children) in changed_bars.iter() {
        match &bar.health_state {
            Some(state) => {
                let Some(child) = children.first() else {
                    continue;
                };
                let mut image_node = node_query.get_mut(*child).unwrap();
                let fill_image = match state {
                    HealthState::Full => &bar_assets.fill_dark_green,
                    HealthState::AmostFull => &bar_assets.fill_light_green,
                    HealthState::Half => &bar_assets.fill_yellow,
                    HealthState::Low => &bar_assets.fill_red,
                    HealthState::Lowest => &bar_assets.fill_dark_red,
                };
                image_node.image = fill_image.clone();
            }
            None => continue,
        }
    }
}

pub fn update_mana(
    mut bar_query: Query<&mut BarUI>,
    mana_query: Query<&Mana, (Changed<Mana>, With<Player>)>,
    bar_entities: Res<BarEntities>,
) {
    let Ok(mana) = mana_query.single() else {
        return;
    };
    let Ok(mut bar) = bar_query.get_mut(bar_entities.mana) else {
        return;
    };
    bar.current = mana.current;
    bar.max = mana.max;
}

pub fn update_health(
    mut bar_query: Query<&mut BarUI>,
    health_query: Query<&Health, (Changed<Health>, With<Player>)>,
    bar_entities: Res<BarEntities>,
) {
    let Ok(health) = health_query.single() else {
        return;
    };
    let Ok(mut bar) = bar_query.get_mut(bar_entities.health) else {
        return;
    };
    bar.current = health.current;
    bar.max = health.max;
    bar.health_state = Some(health.state());
}

// pub fn update_experience(
//     mut bar_query: Query<&mut BarUI>,
//     player_query: Query<&Player, Changed<Player>>,
//     bar_entities: Res<BarEntities>,
// ) {
// let Ok(player) = player_query.single() else {
//     return;
// };
// let Ok(mut bar) = bar_query.get_mut(bar_entities.experience) else {
//     return;
// };
// bar.current = player.max_experience;
// bar.max = player.experience;
// }
