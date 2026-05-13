use bevy::prelude::*;

use crate::agent::components::{Agent, AgentHud, HealthState, Hud};
use crate::agent::{DisplayName, Health, HudBar, Mana};
use crate::camera::GameCamera;
use crate::conf::viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH};
use crate::game_ui::GameViewport;

pub fn update_hud_positions(
    game_cam_q: Query<&GlobalTransform, With<GameCamera>>,
    agents_q: Query<(&GlobalTransform, &AgentHud), With<Agent>>,
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

    for (agent_gt, display_name) in agents_q.iter() {
        let world_pos = agent_gt.translation().truncate();

        // Normalize to [0, 1] UV within the game view (mirrors update_hover_state in reverse).
        let uv = Vec2::new(
            (world_pos.x - cam_pos.x) / GAME_VIEW_WIDTH + 0.5,
            (0.5 - (world_pos.y - cam_pos.y) / GAME_VIEW_HEIGHT)
                - display_name.world_y_offset / GAME_VIEW_HEIGHT,
        );

        // Map UV to logical window pixel position (y-down, top-left origin).
        let screen_px = top_left + uv * size;

        if let Ok(mut tranf) = hud_q.get_mut(display_name.main_entity) {
            tranf.translation =
                Val2::new(Val::Px(screen_px.x.round()), Val::Px(screen_px.y.round()));
        }
    }
}

pub fn update_display_name_health_state(
    actors_q: Query<(&AgentHud, &Health), Changed<Health>>,
    mut state_q: Query<&mut HealthState, With<DisplayName>>,
) {
    for (actor_hud, health) in actors_q.iter() {
        if let Ok(mut state) = state_q.get_mut(actor_hud.display_name) {
            *state = HealthState::from_ratio(health.ratio());
        }
    }
}

pub fn update_display_name_color(
    mut display_names_q: Query<
        (&HealthState, &mut TextColor),
        (With<DisplayName>, Changed<HealthState>),
    >,
) {
    for (health_state, mut color) in display_names_q.iter_mut() {
        color.0 = health_state.color();
    }
}

pub fn update_hud_bar_ratios(
    agents_q: Query<(&AgentHud, Option<&Health>, Option<&Mana>), Changed<Health>>,
    mut hud_bars_q: Query<&mut HudBar>,
) {
    for (agent_hud, health, mana) in agents_q.iter() {
        if let Some(health) = health
            && let Ok(mut hud_bar) = hud_bars_q.get_mut(agent_hud.health_bar.unwrap())
        {
            hud_bar.ratio = health.ratio();
        }
        if let Some(mana) = mana
            && let Ok(mut hud_bar) = hud_bars_q.get_mut(agent_hud.mana_bar.unwrap())
        {
            hud_bar.ratio = mana.ratio();
        }
    }
}

pub fn update_hud_bar_health_state(
    mut health_q: Query<(&HudBar, &mut HealthState), Changed<HudBar>>,
) {
    for (bar, mut state) in health_q.iter_mut() {
        *state = HealthState::from_ratio(bar.ratio);
    }
}

pub fn update_hud_bar_colors(
    mut hud_bars_q: Query<
        (&HealthState, &mut BackgroundColor),
        (With<HudBar>, Changed<HealthState>),
    >,
) {
    for (health_state, mut bg) in hud_bars_q.iter_mut() {
        bg.0 = health_state.color();
    }
}

pub fn resize_hud_fill(mut hud_bars_q: Query<(&mut Node, &HudBar), Changed<HudBar>>) {
    for (mut node, hud_bar) in hud_bars_q.iter_mut() {
        node.width = Val::Percent(hud_bar.ratio * 100.0);
    }
}
