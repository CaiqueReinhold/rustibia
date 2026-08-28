use bevy::prelude::*;

use crate::agent::components::{Agent, AgentHud, HealthState, Hud};
use crate::agent::{DisplayName, Health, HudBar, Mana};
use crate::camera::GameCamera;
use crate::conf::viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH};
use crate::game_ui::{
    GameViewport,
    scaling::{logical_size, snap_to_physical},
};
use crate::map::Position;
use crate::player::components::Player;

pub fn attach_huds_to_viewport(
    mut commands: Commands,
    viewport_q: Query<Entity, With<GameViewport>>,
    orphan_huds: Query<Entity, (With<Hud>, Without<ChildOf>)>,
) {
    let Ok(viewport) = viewport_q.single() else {
        return;
    };
    for hud in orphan_huds.iter() {
        commands.entity(viewport).add_child(hud);
    }
}

/// Places every agent's HUD over its sprite, in viewport-local pixels.
///
/// Reads `Transform`, **not** `GlobalTransform`. This runs in `PostUpdate` before
/// `UiSystems::Layout` — which is where it has to be, because that is the system
/// that consumes the `UiTransform` written below, and `bevy_ui` orders layout
/// *before* `TransformSystems::Propagate`. So at this point `GlobalTransform`
/// still holds last frame's values, while the sprites render from this frame's.
/// Reading it put every label one frame behind the creature it names — invisible
/// on the player, whose offset from the camera never changes, and a visible
/// shimmy on everything else. Agents are children of a floor entity whose
/// transform is only ever identity (`map::floors`), so the two are equal in
/// value; only the freshness differs.
pub fn update_hud_positions(
    mut commands: Commands,
    game_cam_q: Query<&Transform, With<GameCamera>>,
    player_pos_q: Query<&Position, With<Player>>,
    agents_q: Query<(&Transform, &AgentHud, &Position), With<Agent>>,
    mut hud_q: Query<&mut UiTransform, With<Hud>>,
    viewport_q: Query<&ComputedNode, With<GameViewport>>,
) {
    let Ok(game_cam_tf) = game_cam_q.single() else {
        return;
    };
    let Ok(computed) = viewport_q.single() else {
        return;
    };
    let Ok(player_pos) = player_pos_q.single() else {
        return;
    };

    let cam_pos = game_cam_tf.translation.truncate();
    let size = logical_size(computed);

    for (agent_tf, display_name, position) in agents_q.iter() {
        if position.z != player_pos.z {
            commands
                .entity(display_name.main_entity)
                .insert(Visibility::Hidden);
            continue;
        }

        commands
            .entity(display_name.main_entity)
            .insert(Visibility::Visible);

        let world_pos = agent_tf.translation.truncate();

        // Normalize to [0, 1] UV within the game view (mirrors update_hover_state in reverse).
        let uv = Vec2::new(
            (world_pos.x - cam_pos.x) / GAME_VIEW_WIDTH + 0.5,
            (0.5 - (world_pos.y - cam_pos.y) / GAME_VIEW_HEIGHT)
                - display_name.world_y_offset / GAME_VIEW_HEIGHT,
        );

        // HUD nodes are children of the GameViewport, so this is viewport-local
        // (y-down, top-left origin). The viewport's Overflow::clip() then pixel-clips
        // the HUD at the view edge as the agent walks out of frame.
        let local_px = uv * size;

        // Snapped on the physical grid, not the logical one: `Val::Px` is logical,
        // so `.round()` here snapped in steps of the display scale factor and put
        // the label off the grid its own glyphs land on at anything but 100%.
        let snapped = snap_to_physical(computed, local_px);

        if let Ok(mut tranf) = hud_q.get_mut(display_name.main_entity) {
            tranf.translation = Val2::new(Val::Px(snapped.x), Val::Px(snapped.y));
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
