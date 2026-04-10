use bevy::prelude::*;

use crate::actor::components::{Actor, ActorHud, HealthState, Hud};
use crate::actor::{DisplayName, Health, HudBar, Mana};
use crate::camera::GameCamera;
use crate::conf::viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH};
use crate::game_ui::GameViewport;

pub fn update_hud_positions(
    game_cam_q: Query<&GlobalTransform, With<GameCamera>>,
    actors_q: Query<(&GlobalTransform, &ActorHud), With<Actor>>,
    mut hud_q: Query<&mut UiTransform, With<Hud>>,
    viewport_q: Query<(&ComputedNode, &UiGlobalTransform), With<GameViewport>>,
) {
    let Ok(game_cam_gt) = game_cam_q.single() else {
        return;
    };
    let Ok((computed, ui_gt)) = viewport_q.single() else {
        return;
    };

    let cam_pos = game_cam_gt.translation().truncate();
    let size = computed.size();
    let top_left = ui_gt.translation - size * 0.5;

    for (actor_gt, display_name) in actors_q.iter() {
        let world_pos = actor_gt.translation().truncate();

        // Normalize to [0, 1] UV within the game view (mirrors update_hover_state in reverse).
        let uv = Vec2::new(
            (world_pos.x - cam_pos.x) / GAME_VIEW_WIDTH + 0.5,
            (0.5 - (world_pos.y - cam_pos.y) / GAME_VIEW_HEIGHT)
                - display_name.world_y_offset / GAME_VIEW_HEIGHT,
        );

        // Map UV to logical window pixel position (y-down, top-left origin).
        let screen_px = top_left + uv * size;

        if let Ok(mut tranf) = hud_q.get_mut(display_name.main_entity) {
            tranf.translation = Val2::new(Val::Px(screen_px.x.round()), Val::Px(screen_px.y.round()));
        }
    }
}

pub fn update_display_name_health_state(
    mut commands: Commands,
    health_q: Query<(&ActorHud, &Health), Changed<Health>>,
) {
    for (actor_hud, health) in health_q.iter() {
        commands
            .entity(actor_hud.display_name)
            .insert(HealthState::from_ratio(health.ratio()));
    }
}

pub fn update_display_name_color(
    mut commands: Commands,
    display_names_q: Query<(Entity, &HealthState), (With<DisplayName>, Changed<HealthState>)>,
) {
    for (entity, health_state) in display_names_q.iter() {
        commands
            .entity(entity)
            .insert(TextColor(health_state.color()));
    }
}

pub fn update_hud_bar_ratios(
    actors_q: Query<(&ActorHud, Option<&Health>, Option<&Mana>), Changed<Health>>,
    mut hud_bars_q: Query<&mut HudBar>,
) {
    for (actor_hud, health, mana) in actors_q.iter() {
        if let Some(health) = health {
            if let Ok(mut hud_bar) = hud_bars_q.get_mut(actor_hud.health_bar.unwrap()) {
                hud_bar.ratio = health.ratio();
            }
        }
        if let Some(mana) = mana {
            if let Ok(mut hud_bar) = hud_bars_q.get_mut(actor_hud.mana_bar.unwrap()) {
                hud_bar.ratio = mana.ratio();
            }
        }
    }
}

pub fn update_hud_bar_health_state(
    mut commands: Commands,
    health_q: Query<(Entity, &HudBar), (Changed<HudBar>, With<HealthState>)>,
) {
    for (entity, bar) in health_q.iter() {
        commands
            .entity(entity)
            .insert(HealthState::from_ratio(bar.ratio));
    }
}

pub fn update_hud_bar_colors(
    mut commands: Commands,
    display_names_q: Query<(Entity, &HealthState), (With<HudBar>, Changed<HealthState>)>,
) {
    for (entity, health_state) in display_names_q.iter() {
        commands
            .entity(entity)
            .insert(BackgroundColor(health_state.color()));
    }
}

pub fn resize_hud_fill(mut hud_bars_q: Query<(&mut Node, &HudBar), Changed<HudBar>>) {
    for (mut node, hud_bar) in hud_bars_q.iter_mut() {
        node.width = Val::Percent(hud_bar.ratio * 100.0);
    }
}
