//! World-anchored transient text: damage numbers and speech over a tile.
//!
//! Distinct from `core::text`, which anchors to the *viewport* (`ActionDenied` at
//! the bottom, `Look` at the centre). Everything here is pinned to a tile and
//! outlives whatever caused it — a speaker who walks away leaves their text
//! behind, which is what the real Tibia client does.
//!
//! ## Two coordinate spaces
//!
//! This is the part that is invisible at a 1:1 viewport and wrong at every other
//! size. Anchor offsets (tile centre → head height) are **world px**, folded into
//! the UV before the multiply by viewport size, so they scale with the view and
//! stay glued to the sprite — the same thing `AgentHud::world_y_offset` does. Text
//! offsets (rise, collision push, block gap) are **logical px** applied after that
//! conversion, because the font is a fixed size and motion measured against glyph
//! metrics must stay in glyph units.
//!
//! ## Why placement runs before `UiSystems::Layout`
//!
//! `ui_layout_system` is one system that reads `UiTransform`/`Node` and writes both
//! `ComputedNode` and `UiGlobalTransform`. There is no point in the frame where you
//! can read this frame's measured size *and* still influence this frame's layout,
//! so placement reads the **previous** frame's `ComputedNode::size` and writes
//! `Node.left`/`Node.top` before layout runs. A text therefore cannot be placed on
//! the frame it spawned: it carries `Unplaced` and stays hidden until measured.

use std::collections::VecDeque;
use std::time::Duration;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::text::FontSmoothing;
use bevy_text_outline::TextOutline;

use crate::camera::GameCamera;
use crate::conf::floating_text as ft;
use crate::conf::map::TILE_SIZE;
use crate::conf::ui::chat::LOCAL_CHANNEL_COLOR;
use crate::conf::viewport::{GAME_VIEW_HEIGHT, GAME_VIEW_WIDTH};
use crate::game_ui::scaling::logical_size;
use crate::game_ui::{GameUiAssets, GameViewport};
use crate::map::{Map, Position};
use crate::network::events::ShowFloatingText;
use crate::player::components::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatingTextType {
    HitPoints,
    PlayerMessage,
}

/// State shared by both kinds.
#[derive(Component, Debug)]
pub struct FloatingText {
    pub kind: FloatingTextType,
    /// The tile this text is pinned to, for life.
    pub anchor: Position,
    /// `Time::elapsed` at spawn. The collision sort key.
    pub spawned_at: Duration,
    /// Collision-resolved push, logical px, upward. Vertical only.
    pub offset_y: f32,
}

/// Present until the text has been measured and placed. While it is present the
/// text stays hidden, because a node of size `(0, 0)` cannot be centred on its
/// anchor.
#[derive(Component, Debug)]
pub struct Unplaced;

#[derive(Component, Debug)]
pub struct HitPointsText {
    /// Parsed once at spawn. `None` means the text is not a number and can never
    /// merge, so the merge path never re-parses.
    pub value: Option<i64>,
    pub color: Color,
    pub timer: Timer,
}

#[derive(Component, Debug)]
pub struct SpeechBlock {
    pub lines: VecDeque<(String, Timer)>,
}

impl SpeechBlock {
    pub fn compose(&self) -> String {
        self.lines
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The colour the server asked for, or this kind's default.
pub fn resolve_color(kind: FloatingTextType, color: Option<(u8, u8, u8)>) -> Color {
    match color {
        Some((r, g, b)) => Color::srgb_u8(r, g, b),
        None => match kind {
            FloatingTextType::HitPoints => Color::WHITE,
            FloatingTextType::PlayerMessage => Color::from(LOCAL_CHANNEL_COLOR),
        },
    }
}

/// How long one line of speech stays up. Longer messages last longer, floored so a
/// one-word line is still readable and capped so a 255-character line does not sit
/// there for fifteen seconds.
pub fn line_duration(chars: usize) -> Duration {
    let ms = (ft::SPEECH_MS_PER_CHAR * chars as u64).clamp(ft::SPEECH_MIN_MS, ft::SPEECH_MAX_MS);
    Duration::from_millis(ms)
}

/// Logical px a damage number has risen at `fraction` of its life.
pub fn risen(fraction: f32) -> f32 {
    ft::HP_RISE_PX * fraction
}

/// Opacity at `fraction` of life: opaque, then a linear ramp over the tail.
///
/// The clamp is defensive only — the sole caller passes `Timer::fraction()`,
/// which Bevy keeps within `[0.0, 1.0]` by construction.
pub fn alpha(fraction: f32) -> f32 {
    if fraction < ft::HP_FADE_START {
        return 1.0;
    }
    ((1.0 - fraction) / (1.0 - ft::HP_FADE_START)).clamp(0.0, 1.0)
}

/// Where to place an arriving damage number, given the current heights above the
/// tile of every live number already there.
///
/// Sits one clearance above the highest of them, so simultaneous arrivals stack
/// instead of colliding. Two escapes keep the column bounded: if nothing is still
/// within one clearance of the tile the bottom slot is reused, and past
/// `HP_MAX_STAGGER_PX` the column recycles to the bottom rather than marching off
/// the top of the view.
pub fn stagger_offset(heights: &[f32]) -> f32 {
    // Vacuously true for an empty slice, which is what makes the no-neighbours
    // case fall out of this branch rather than needing one of its own.
    if heights.iter().all(|h| *h >= ft::HP_CLEARANCE_PX) {
        return 0.0;
    }
    let highest = heights.iter().copied().fold(0.0f32, f32::max);
    let candidate = highest + ft::HP_CLEARANCE_PX;
    if candidate > ft::HP_MAX_STAGGER_PX {
        return 0.0;
    }
    candidate
}

/// A live damage number reduced to what arrival planning needs. Collected from the
/// world before any mutation, so the planner is a pure function.
pub struct LiveHitPoints {
    pub entity: Entity,
    pub offset_y: f32,
    pub value: Option<i64>,
    pub color: Color,
    pub fraction: f32,
    pub spawned_at: Duration,
}

pub enum HpArrival {
    /// Fold the arriving number into an existing one.
    Merge { target: Entity, sum: i64 },
    /// Spawn a new number at this vertical offset.
    Spawn { offset_y: f32 },
}

/// Decide what an arriving damage number does. `live` must already be filtered to
/// the numbers sharing the arriving text's tile.
///
/// Merge candidates are considered newest first, so a burst collapses into the
/// freshest number, but an older text is still used when the newest cannot absorb
/// the hit.
pub fn plan_hit_points(live: &[LiveHitPoints], value: Option<i64>, color: Color) -> HpArrival {
    if let Some(v) = value {
        let mut newest_first: Vec<&LiveHitPoints> = live.iter().collect();
        newest_first.sort_by_key(|o| std::cmp::Reverse(o.spawned_at));
        for other in newest_first {
            if other.color == color
                && other.fraction < ft::HP_MERGE_WINDOW
                && let Some(existing) = other.value
            {
                return HpArrival::Merge {
                    target: other.entity,
                    sum: existing + v,
                };
            }
        }
    }

    let heights: Vec<f32> = live
        .iter()
        .map(|o| o.offset_y + risen(o.fraction))
        .collect();
    HpArrival::Spawn {
        offset_y: stagger_offset(&heights),
    }
}

/// A speech block reduced to what collision resolution needs. Viewport-local
/// logical px, y-down, with `anchor_px` the block's bottom-centre.
pub struct BlockLayout {
    pub anchor_px: Vec2,
    pub size: Vec2,
    pub spawned_at: Duration,
}

fn block_rect(b: &BlockLayout, offset_y: f32) -> Rect {
    // y-down: pushing a block up subtracts from its bottom edge.
    let bottom = b.anchor_px.y - offset_y;
    Rect {
        min: Vec2::new(b.anchor_px.x - b.size.x / 2.0, bottom - b.size.y),
        max: Vec2::new(b.anchor_px.x + b.size.x / 2.0, bottom),
    }
}

/// Strict overlap: blocks whose edges merely touch are left alone.
fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// One `offset_y` per input block, in input order.
///
/// Oldest first, and the oldest holds its position; each newer block is pushed up
/// until it clears every block already placed. Vertical only, matching Tibia.
pub fn resolve_offsets(blocks: &[BlockLayout]) -> Vec<f32> {
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    order.sort_by_key(|&i| blocks[i].spawned_at);

    let mut offsets = vec![0.0f32; blocks.len()];
    let mut placed: Vec<Rect> = Vec::with_capacity(blocks.len());

    for &i in &order {
        let b = &blocks[i];
        let mut offset_y = 0.0f32;
        // Every push clears one already-placed rect outright, so this cannot need
        // more iterations than there are rects placed. The bound also makes a
        // degenerate zero-height block terminate instead of spinning.
        for _ in 0..=placed.len() {
            let rect = block_rect(b, offset_y);
            let Some(blocker) = placed.iter().find(|p| overlaps(&rect, p)) else {
                break;
            };
            offset_y += (rect.max.y - blocker.min.y) + ft::SPEECH_GAP_PX;
        }
        offsets[i] = offset_y;
        placed.push(block_rect(b, offset_y));
    }

    offsets
}

/// Turns an arriving `ShowFloatingText` into a new entity, a merged number, or an
/// extra line on an existing speech block.
pub fn on_floating_text(
    event: On<ShowFloatingText>,
    mut commands: Commands,
    time: Res<Time>,
    ui_assets: Res<GameUiAssets>,
    map: Res<Map>,
    viewport_q: Query<Entity, With<GameViewport>>,
    agent_pos_q: Query<&Position>,
    // `Without` on both sides is what makes these two `&mut Text` queries provably
    // disjoint; without it Bevy panics at runtime on conflicting access.
    mut hp_q: Query<(Entity, &FloatingText, &mut HitPointsText, &mut Text), Without<SpeechBlock>>,
    mut speech_q: Query<
        (Entity, &FloatingText, &mut SpeechBlock, &mut Text),
        Without<HitPointsText>,
    >,
) {
    let Ok(viewport) = viewport_q.single() else {
        return;
    };
    let Some(anchor) = map
        .get_agent(event.agent_id)
        .and_then(|entity| agent_pos_q.get(entity).ok())
        .cloned()
    else {
        return;
    };
    let now = time.elapsed();
    let color = resolve_color(event.text_type, event.color);

    match event.text_type {
        FloatingTextType::HitPoints => {
            let value = event.text.trim().parse::<i64>().ok();

            // Snapshot before any mutation, so planning stays a pure function.
            let live: Vec<LiveHitPoints> = hp_q
                .iter()
                .filter(|(_, ft, _, _)| ft.anchor == anchor)
                .map(|(entity, ft, hp, _)| LiveHitPoints {
                    entity,
                    offset_y: ft.offset_y,
                    value: hp.value,
                    color: hp.color,
                    fraction: hp.timer.fraction(),
                    spawned_at: ft.spawned_at,
                })
                .collect();

            match plan_hit_points(&live, value, color) {
                HpArrival::Merge { target, sum } => {
                    if let Ok((_, _, mut hp, mut text)) = hp_q.get_mut(target) {
                        hp.value = Some(sum);
                        text.0 = sum.to_string();
                        // The timer is deliberately not restarted: a sustained
                        // stream of hits must not produce an immortal number.
                    }
                }
                HpArrival::Spawn { offset_y } => {
                    commands.spawn((
                        FloatingText {
                            kind: FloatingTextType::HitPoints,
                            anchor: anchor.clone(),
                            spawned_at: now,
                            offset_y,
                        },
                        HitPointsText {
                            value,
                            color,
                            timer: Timer::new(
                                Duration::from_millis(ft::HP_DURATION_MS),
                                TimerMode::Once,
                            ),
                        },
                        Unplaced,
                        Visibility::Hidden,
                        ChildOf(viewport),
                        RenderLayers::layer(1),
                        ZIndex(ft::Z_INDEX),
                        text_node(),
                        Text::new(event.text.clone()),
                        text_font(&ui_assets),
                        TextColor(color),
                        TextOutline {
                            width: ft::OUTLINE_WIDTH,
                            ..default()
                        },
                    ));
                }
            }
        }
        FloatingTextType::PlayerMessage => {
            let existing = speech_q
                .iter()
                .find(|(_, ft, _, _)| ft.anchor == anchor)
                .map(|(entity, _, _, _)| entity);

            let line = (
                event.text.clone(),
                Timer::new(line_duration(event.text.chars().count()), TimerMode::Once),
            );

            if let Some(entity) = existing
                && let Ok((_, _, mut block, mut text)) = speech_q.get_mut(entity)
            {
                block.lines.push_back(line);
                while block.lines.len() > ft::SPEECH_MAX_LINES {
                    block.lines.pop_front();
                }
                text.0 = block.compose();
                return;
            }

            let mut lines = VecDeque::new();
            lines.push_back(line);
            let composed = event.text.clone();
            commands.spawn((
                FloatingText {
                    kind: FloatingTextType::PlayerMessage,
                    anchor: anchor.clone(),
                    spawned_at: now,
                    offset_y: 0.0,
                },
                SpeechBlock { lines },
                Unplaced,
                Visibility::Hidden,
                ChildOf(viewport),
                RenderLayers::layer(1),
                ZIndex(ft::Z_INDEX),
                speech_node(),
                Text::new(composed),
                TextLayout::new_with_justify(Justify::Center),
                text_font(&ui_assets),
                TextColor(color),
                TextOutline {
                    width: ft::OUTLINE_WIDTH,
                    ..default()
                },
            ));
        }
    }
}

fn tile_centre(anchor: &Position) -> Vec2 {
    let world = anchor.to_world();
    Vec2::new(world.x + TILE_SIZE / 2.0, world.y - TILE_SIZE / 2.0)
}

fn anchor_px(anchor: &Position, kind: FloatingTextType, cam_pos: Vec2, size: Vec2) -> Vec2 {
    let world = tile_centre(anchor);
    let world_y_offset = match kind {
        FloatingTextType::HitPoints => 0.0,
        FloatingTextType::PlayerMessage => ft::SPEECH_HEAD_OFFSET_WORLD,
    };
    let uv = Vec2::new(
        (world.x - cam_pos.x) / GAME_VIEW_WIDTH + 0.5,
        (0.5 - (world.y - cam_pos.y) / GAME_VIEW_HEIGHT) - world_y_offset / GAME_VIEW_HEIGHT,
    );
    uv * size
}

fn text_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        ..default()
    }
}

fn speech_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        max_width: Val::Px(ft::SPEECH_MAX_WIDTH_PX),
        ..default()
    }
}

fn text_font(ui_assets: &GameUiAssets) -> TextFont {
    TextFont {
        font: ui_assets.font.clone(),
        font_size: ft::FONT_SIZE,
        ..default()
    }
    .with_font_smoothing(FontSmoothing::None)
}

/// Advances each number's timer, fades its tail, and despawns it at the end.
pub fn tick_hit_points(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut HitPointsText, &mut TextColor)>,
) {
    for (entity, mut hp, mut color) in q.iter_mut() {
        hp.timer.tick(time.delta());
        if hp.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        color.0 = hp.color.with_alpha(alpha(hp.timer.fraction()));
    }
}

/// Expires speech lines, rebuilds the composed text when the line set changed, and
/// despawns a block once its last line is gone.
///
/// The text is rewritten **only** when a line actually left, because the collision
/// pass keys its dirty check on `Changed<Text>` — an unconditional write would make
/// it re-resolve every frame.
pub fn tick_speech_blocks(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut SpeechBlock, &mut Text)>,
) {
    for (entity, mut block, mut text) in q.iter_mut() {
        let before = block.lines.len();
        for (_, timer) in block.lines.iter_mut() {
            timer.tick(time.delta());
        }
        block.lines.retain(|(_, timer)| !timer.is_finished());

        if block.lines.is_empty() {
            commands.entity(entity).despawn();
            continue;
        }
        if block.lines.len() != before {
            text.0 = block.compose();
        }
    }
}

/// Pushes overlapping speech blocks clear of each other and releases newly
/// measured blocks for display.
///
/// Sizes come from the **previous** frame's layout (see the module docs), so a
/// block spawned this frame is placed on the next one.
pub fn resolve_speech_collisions(
    mut commands: Commands,
    game_cam_q: Query<&GlobalTransform, With<GameCamera>>,
    player_pos_q: Query<&Position, With<Player>>,
    viewport_q: Query<&ComputedNode, With<GameViewport>>,
    mut blocks_q: Query<(Entity, &mut FloatingText, &ComputedNode), With<SpeechBlock>>,
    // `Changed<Text>` and not `Changed<SpeechBlock>`: ticking the line timers
    // touches `SpeechBlock` every frame, so it is always "changed" and would gate
    // nothing. `Text` is written only when a line is added or expires.
    changed_q: Query<(), (Changed<Text>, With<SpeechBlock>)>,
    unplaced_q: Query<(), (With<SpeechBlock>, With<Unplaced>)>,
    viewport_resized_q: Query<(), (Changed<ComputedNode>, With<GameViewport>)>,
    mut removed: RemovedComponents<SpeechBlock>,
) {
    // Drained unconditionally and first: `||` short-circuits, and an undrained
    // `RemovedComponents` reader accumulates events forever.
    let removed_any = removed.read().count() > 0;
    let dirty = removed_any
        || !changed_q.is_empty()
        || !unplaced_q.is_empty()
        || !viewport_resized_q.is_empty();
    if !dirty {
        return;
    }

    let Ok(cam) = game_cam_q.single() else {
        return;
    };
    let Ok(player_pos) = player_pos_q.single() else {
        return;
    };
    let Ok(viewport) = viewport_q.single() else {
        return;
    };
    let cam_pos = cam.translation().truncate();
    let size = logical_size(viewport);

    // Only blocks on the player's floor compete for space, and a block with no
    // measured size yet cannot be placed at all.
    let mut entities = Vec::new();
    let mut layouts = Vec::new();
    for (entity, text, node) in blocks_q.iter() {
        let node_size = logical_size(node);
        if text.anchor.z != player_pos.z || node_size.x <= 0.0 || node_size.y <= 0.0 {
            continue;
        }
        entities.push(entity);
        layouts.push(BlockLayout {
            anchor_px: anchor_px(&text.anchor, text.kind, cam_pos, size),
            size: node_size,
            spawned_at: text.spawned_at,
        });
    }

    let offsets = resolve_offsets(&layouts);
    for (entity, offset_y) in entities.into_iter().zip(offsets) {
        if let Ok((_, mut text, _)) = blocks_q.get_mut(entity) {
            text.offset_y = offset_y;
        }
        commands.entity(entity).remove::<Unplaced>();
    }
}

/// Writes every floating text's `Node` position, hides off-floor and unmeasured
/// text, and applies the rise animation.
///
/// Placement goes through `Node.left`/`Node.top` rather than `UiTransform` because
/// the text must be *centred* on its anchor, which needs its measured width — and
/// `UiTransform` is consumed by the same system that produces that measurement.
pub fn position_floating_texts(
    mut commands: Commands,
    game_cam_q: Query<&GlobalTransform, With<GameCamera>>,
    player_pos_q: Query<&Position, With<Player>>,
    viewport_q: Query<&ComputedNode, With<GameViewport>>,
    mut texts_q: Query<(
        Entity,
        &FloatingText,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
        Option<&HitPointsText>,
        Option<&Unplaced>,
    )>,
) {
    let Ok(cam) = game_cam_q.single() else {
        return;
    };
    let Ok(player_pos) = player_pos_q.single() else {
        return;
    };
    let Ok(viewport) = viewport_q.single() else {
        return;
    };
    let cam_pos = cam.translation().truncate();
    let size = logical_size(viewport);

    for (entity, text, node, mut style, mut visibility, hp, unplaced) in texts_q.iter_mut() {
        let node_size = logical_size(node);
        let measured = node_size.x > 0.0 && node_size.y > 0.0;

        // Off the player's floor, or not yet measured, means nothing to show.
        let want = if text.anchor.z == player_pos.z && measured && unplaced.is_none() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        if !measured {
            continue;
        }

        // A damage number is placed the frame it is measured; a speech block waits
        // for the collision pass to clear its `Unplaced`.
        if unplaced.is_some() && hp.is_some() {
            commands.entity(entity).remove::<Unplaced>();
        }

        let anchor = anchor_px(&text.anchor, text.kind, cam_pos, size);
        let rise = hp.map(|hp| risen(hp.timer.fraction())).unwrap_or(0.0);

        style.left = Val::Px((anchor.x - node_size.x / 2.0).round());
        style.top = Val::Px((anchor.y - node_size.y - text.offset_y - rise).round());
    }
}

/// Dev-only floating text, driven from the keyboard instead of the server.
///
/// - `F9` — a number over the player in a fixed colour: repeated presses
///   exercise **merging**.
/// - `F10` — a number over the player in a rotating colour: repeated presses
///   exercise **staggering**.
/// - `F11` — speech over the player: repeated presses exercise **queueing**
///   into one block.
/// - `F12` — speech over the nearest other agent: exercises **cross-tile push**
///   against a block on the player's own tile. A no-op with nobody else in
///   view, since the message names an agent and no longer names a tile.
#[cfg(feature = "debug")]
pub fn debug_spawn_floating_text(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<(&Player, &Position)>,
    others_q: Query<(&crate::agent::Agent, &Position), Without<Player>>,
    mut nonce: Local<u32>,
) {
    let Ok((player, player_pos)) = player_q.single() else {
        return;
    };

    let merge = keys.just_pressed(KeyCode::F9);
    let stagger = keys.just_pressed(KeyCode::F10);
    let speech = keys.just_pressed(KeyCode::F11);
    let neighbour = keys.just_pressed(KeyCode::F12);
    if !merge && !stagger && !speech && !neighbour {
        return;
    }

    // A one-line LCG, so the trigger needs no `rand` dependency.
    *nonce = nonce.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let r = (*nonce >> 16) as usize;

    if merge {
        commands.trigger(ShowFloatingText {
            text: format!("-{}", 1 + r % 99),
            agent_id: player.agent_id,
            text_type: FloatingTextType::HitPoints,
            color: Some((255, 64, 64)),
        });
    }

    if stagger {
        const COLORS: [(u8, u8, u8); 4] = [
            (255, 64, 64),
            (64, 200, 255),
            (255, 255, 64),
            (200, 96, 255),
        ];
        commands.trigger(ShowFloatingText {
            text: format!("-{}", 1 + r % 99),
            agent_id: player.agent_id,
            text_type: FloatingTextType::HitPoints,
            color: Some(COLORS[r % COLORS.len()]),
        });
    }

    if speech || neighbour {
        const LINES: [&str; 4] = [
            "hi",
            "hello there",
            "exura vita",
            "a deliberately long sentence to exercise the wrap width",
        ];
        // One key per speaker, so queueing and cross-tile push can be exercised
        // separately rather than at random.
        let speaker = if speech {
            Some(player.agent_id)
        } else {
            nearest_other_agent(player_pos, &others_q)
        };
        let Some(agent_id) = speaker else {
            warn!("F12 needs another agent in view to speak from");
            return;
        };
        commands.trigger(ShowFloatingText {
            text: LINES[r % LINES.len()].to_owned(),
            agent_id,
            text_type: FloatingTextType::PlayerMessage,
            color: None,
        });
    }
}

/// The agent nearest the player on the player's own floor, if any.
#[cfg(feature = "debug")]
fn nearest_other_agent(
    player_pos: &Position,
    others_q: &Query<(&crate::agent::Agent, &Position), Without<Player>>,
) -> Option<crate::agent::AgentId> {
    others_q
        .iter()
        .filter(|(_, pos)| pos.z == player_pos.z)
        .min_by_key(|(_, pos)| {
            pos.x.abs_diff(player_pos.x) as u32 + pos.y.abs_diff(player_pos.y) as u32
        })
        .map(|(agent, _)| agent.agent_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_colour_wins_over_the_default() {
        assert_eq!(
            resolve_color(FloatingTextType::HitPoints, Some((10, 20, 30))),
            Color::srgb_u8(10, 20, 30)
        );
    }

    #[test]
    fn hit_points_default_to_white() {
        assert_eq!(
            resolve_color(FloatingTextType::HitPoints, None),
            Color::WHITE
        );
    }

    /// Speech reuses the local-chat yellow so a block and its chat-log line match.
    #[test]
    fn speech_defaults_to_the_local_chat_colour() {
        assert_eq!(
            resolve_color(FloatingTextType::PlayerMessage, None),
            Color::from(LOCAL_CHANNEL_COLOR)
        );
    }

    #[test]
    fn line_duration_is_clamped_at_both_ends() {
        assert_eq!(line_duration(1), Duration::from_millis(ft::SPEECH_MIN_MS));
        assert_eq!(line_duration(255), Duration::from_millis(ft::SPEECH_MAX_MS));
    }

    #[test]
    fn line_duration_scales_between_the_bounds() {
        // 100 chars × 60 ms = 6000 ms, inside [3000, 8000].
        assert_eq!(line_duration(100), Duration::from_millis(6000));
    }

    /// `risen` had no direct test until mutation testing found that dropping the
    /// `HP_RISE_PX` multiply entirely left the whole suite green. It feeds both the
    /// stagger heights and the rise animation, so a silent regression here would
    /// corrupt both.
    #[test]
    fn a_number_rises_the_full_distance_over_its_life() {
        assert_eq!(risen(0.0), 0.0);
        assert_eq!(risen(0.5), ft::HP_RISE_PX / 2.0);
        assert_eq!(risen(1.0), ft::HP_RISE_PX);
    }

    #[test]
    fn alpha_is_opaque_until_the_fade_starts() {
        assert_eq!(alpha(0.0), 1.0);
        assert_eq!(alpha(ft::HP_FADE_START - 0.01), 1.0);
    }

    #[test]
    fn alpha_reaches_zero_at_the_end_of_life() {
        assert_eq!(alpha(1.0), 0.0);
        assert!(alpha(0.9) > 0.0 && alpha(0.9) < 1.0);
    }

    #[test]
    fn the_first_text_at_a_tile_is_not_staggered() {
        assert_eq!(stagger_offset(&[]), 0.0);
    }

    #[test]
    fn a_second_text_sits_one_clearance_above_the_first() {
        assert_eq!(stagger_offset(&[0.0]), ft::HP_CLEARANCE_PX);
    }

    /// The property OTClient's formula loses. It computes `CLEARANCE - risen`,
    /// putting the newcomer at `12 - risen` while the older sits at `risen` — equal
    /// at `risen == 6`, so the two texts pass through each other in the middle of
    /// the window the rule exists to protect.
    #[test]
    fn staggered_texts_keep_a_full_clearance_at_every_point_in_the_window() {
        let mut risen = 0.0;
        while risen < ft::HP_CLEARANCE_PX {
            let newcomer = stagger_offset(&[risen]);
            assert!(
                (newcomer - risen) >= ft::HP_CLEARANCE_PX - f32::EPSILON,
                "gap collapsed to {} at risen {risen}",
                newcomer - risen
            );
            risen += 1.0;
        }
    }

    /// Three texts in the same instant. Comparing only against the newest would put
    /// the third back at zero, on top of the first.
    #[test]
    fn a_third_simultaneous_text_clears_both_predecessors() {
        assert_eq!(stagger_offset(&[0.0, ft::HP_CLEARANCE_PX]), 24.0);
    }

    /// The last slot the column is allowed to use: a candidate of *exactly*
    /// `HP_MAX_STAGGER_PX` must still be taken. Without this the cap comparison
    /// could be widened to `>=` and nothing would notice.
    #[test]
    fn the_last_slot_below_the_cap_is_still_used() {
        assert_eq!(stagger_offset(&[0.0, 12.0, 24.0]), ft::HP_MAX_STAGGER_PX);
    }

    #[test]
    fn the_column_recycles_to_the_bottom_past_the_cap() {
        // 0, 12, 24, 36 are occupied; 48 would exceed HP_MAX_STAGGER_PX.
        assert_eq!(stagger_offset(&[0.0, 12.0, 24.0, 36.0]), 0.0);
    }

    /// Once every live text has risen clear of the bottom slot, reuse it rather
    /// than stacking forever.
    #[test]
    fn the_bottom_slot_is_reused_once_it_is_free() {
        assert_eq!(stagger_offset(&[12.0, 24.0]), 0.0);
    }

    fn live(
        entity: Entity,
        value: Option<i64>,
        color: Color,
        fraction: f32,
        ms: u64,
    ) -> LiveHitPoints {
        LiveHitPoints {
            entity,
            offset_y: 0.0,
            value,
            color,
            fraction,
            spawned_at: Duration::from_millis(ms),
        }
    }

    #[test]
    fn the_first_number_at_a_tile_just_spawns() {
        match plan_hit_points(&[], Some(5), Color::WHITE) {
            HpArrival::Spawn { offset_y } => assert_eq!(offset_y, 0.0),
            HpArrival::Merge { .. } => panic!("nothing to merge with"),
        }
    }

    #[test]
    fn merge_sums_two_numbers_of_the_same_colour() {
        let existing = Entity::from_raw_u32(1).unwrap();
        let live = [live(existing, Some(-12), Color::WHITE, 0.1, 0)];
        match plan_hit_points(&live, Some(-8), Color::WHITE) {
            HpArrival::Merge { target, sum } => {
                assert_eq!(target, existing);
                assert_eq!(sum, -20);
            }
            HpArrival::Spawn { .. } => panic!("expected a merge"),
        }
    }

    #[test]
    fn a_different_colour_does_not_merge() {
        let live = [live(
            Entity::from_raw_u32(1).unwrap(),
            Some(12),
            Color::WHITE,
            0.0,
            0,
        )];
        match plan_hit_points(&live, Some(8), Color::BLACK) {
            HpArrival::Spawn { offset_y } => {
                assert_eq!(offset_y, ft::HP_CLEARANCE_PX, "must be staggered clear")
            }
            HpArrival::Merge { .. } => panic!("different colours must not merge"),
        }
    }

    #[test]
    fn a_non_numeric_arrival_never_merges() {
        let live = [live(
            Entity::from_raw_u32(1).unwrap(),
            Some(12),
            Color::WHITE,
            0.0,
            0,
        )];
        assert!(matches!(
            plan_hit_points(&live, None, Color::WHITE),
            HpArrival::Spawn { .. }
        ));
    }

    #[test]
    fn a_non_numeric_existing_text_is_not_a_merge_target() {
        let live = [live(
            Entity::from_raw_u32(1).unwrap(),
            None,
            Color::WHITE,
            0.0,
            0,
        )];
        assert!(matches!(
            plan_hit_points(&live, Some(8), Color::WHITE),
            HpArrival::Spawn { .. }
        ));
    }

    #[test]
    fn merging_is_refused_past_the_window() {
        let live = [live(
            Entity::from_raw_u32(1).unwrap(),
            Some(12),
            Color::WHITE,
            ft::HP_MERGE_WINDOW + 0.01,
            0,
        )];
        assert!(matches!(
            plan_hit_points(&live, Some(8), Color::WHITE),
            HpArrival::Spawn { .. }
        ));
    }

    /// Two candidates, only the older one mergeable. Merging must still happen —
    /// picking only the newest would spawn a redundant number next to a text that
    /// was happy to absorb it.
    #[test]
    fn an_older_mergeable_text_is_used_when_the_newest_cannot_merge() {
        let old = Entity::from_raw_u32(1).unwrap();
        let new = Entity::from_raw_u32(2).unwrap();
        let live = [
            live(old, Some(3), Color::WHITE, 0.1, 0),
            live(new, Some(4), Color::BLACK, 0.1, 10),
        ];
        match plan_hit_points(&live, Some(5), Color::WHITE) {
            HpArrival::Merge { target, sum } => {
                assert_eq!(target, old);
                assert_eq!(sum, 8);
            }
            HpArrival::Spawn { .. } => panic!("the older text was mergeable"),
        }
    }

    /// With two mergeable candidates the freshest wins, so a burst collapses into
    /// the number the player is currently looking at.
    #[test]
    fn the_newest_mergeable_text_wins() {
        let old = Entity::from_raw_u32(1).unwrap();
        let new = Entity::from_raw_u32(2).unwrap();
        let live = [
            live(old, Some(3), Color::WHITE, 0.1, 0),
            live(new, Some(4), Color::WHITE, 0.1, 10),
        ];
        match plan_hit_points(&live, Some(5), Color::WHITE) {
            HpArrival::Merge { target, sum } => {
                assert_eq!(target, new);
                assert_eq!(sum, 9);
            }
            HpArrival::Spawn { .. } => panic!("expected a merge"),
        }
    }

    fn block(x: f32, y: f32, w: f32, h: f32, ms: u64) -> BlockLayout {
        BlockLayout {
            anchor_px: Vec2::new(x, y),
            size: Vec2::new(w, h),
            spawned_at: Duration::from_millis(ms),
        }
    }

    #[test]
    fn a_lone_block_is_not_pushed() {
        assert_eq!(
            resolve_offsets(&[block(100.0, 100.0, 40.0, 12.0, 0)]),
            [0.0]
        );
    }

    #[test]
    fn blocks_far_apart_are_not_pushed() {
        let blocks = [
            block(0.0, 100.0, 40.0, 12.0, 0),
            block(300.0, 100.0, 40.0, 12.0, 1),
        ];
        assert_eq!(resolve_offsets(&blocks), [0.0, 0.0]);
    }

    #[test]
    fn overlapping_blocks_are_pushed_apart() {
        // Same anchor, so the newer block overlaps the older exactly.
        let blocks = [
            block(100.0, 100.0, 40.0, 12.0, 0),
            block(100.0, 100.0, 40.0, 12.0, 1),
        ];
        let offsets = resolve_offsets(&blocks);
        assert_eq!(offsets[0], 0.0, "the oldest block holds its position");
        assert_eq!(
            offsets[1],
            12.0 + ft::SPEECH_GAP_PX,
            "the newer block clears the older by its height plus the gap"
        );
    }

    /// Input order is not spawn order. The oldest wins regardless of where it sits
    /// in the slice, and offsets come back in input order.
    #[test]
    fn the_oldest_block_holds_its_position_whatever_the_input_order() {
        let blocks = [
            block(100.0, 100.0, 40.0, 12.0, 50), // newer, listed first
            block(100.0, 100.0, 40.0, 12.0, 10), // older
        ];
        let offsets = resolve_offsets(&blocks);
        assert_eq!(offsets[1], 0.0, "the older block is the anchor");
        assert_eq!(offsets[0], 12.0 + ft::SPEECH_GAP_PX);
    }

    #[test]
    fn a_third_block_clears_both_predecessors() {
        let blocks = [
            block(100.0, 100.0, 40.0, 12.0, 0),
            block(100.0, 100.0, 40.0, 12.0, 1),
            block(100.0, 100.0, 40.0, 12.0, 2),
        ];
        let offsets = resolve_offsets(&blocks);
        assert_eq!(offsets[2], 2.0 * (12.0 + ft::SPEECH_GAP_PX));
    }

    /// Horizontally adjacent but not overlapping: touching edges must not count as
    /// an overlap, or every block in a row would be pushed.
    #[test]
    fn blocks_that_only_touch_are_not_pushed() {
        let blocks = [
            block(0.0, 100.0, 40.0, 12.0, 0),
            block(40.0, 100.0, 40.0, 12.0, 1),
        ];
        assert_eq!(resolve_offsets(&blocks), [0.0, 0.0]);
    }

    /// The vertical mirror of `blocks_that_only_touch_are_not_pushed`. Without it
    /// nothing pins the y-axis clauses of `overlaps` at a boundary: the horizontal
    /// test short-circuits on its first clause, so a `<` widened to `<=` on either
    /// y comparison would go unnoticed. Block B's bottom edge sits exactly on block
    /// A's top edge.
    #[test]
    fn blocks_that_only_touch_vertically_are_not_pushed() {
        let blocks = [
            block(100.0, 100.0, 40.0, 12.0, 0), // occupies y 88..100
            block(100.0, 88.0, 40.0, 12.0, 1),  // occupies y 76..88
        ];
        assert_eq!(resolve_offsets(&blocks), [0.0, 0.0]);
    }

    /// The mirror of `blocks_that_only_touch_are_not_pushed`, for the other x
    /// clause. Short-circuit `&&` means the original test never evaluates
    /// `b.min.x < a.max.x` at all; placing the newer block to the *left* is what
    /// puts that comparison on the boundary.
    #[test]
    fn blocks_that_only_touch_horizontally_from_the_left_are_not_pushed() {
        let blocks = [
            block(40.0, 100.0, 40.0, 12.0, 0), // occupies x 20..60
            block(0.0, 100.0, 40.0, 12.0, 1),  // occupies x -20..20
        ];
        assert_eq!(resolve_offsets(&blocks), [0.0, 0.0]);
    }

    /// The mirror of the above, for the *other* y clause. `overlaps` compares the
    /// two rects in both directions, and one touching case only pins one clause:
    /// with B above A it is `b.min.y < a.max.y`, with B below A it is
    /// `a.min.y < b.max.y`. Both are needed or half the boundary goes unwatched.
    #[test]
    fn blocks_that_only_touch_vertically_from_below_are_not_pushed() {
        let blocks = [
            block(100.0, 100.0, 40.0, 12.0, 0), // occupies y 88..100
            block(100.0, 112.0, 40.0, 12.0, 1), // occupies y 100..112
        ];
        assert_eq!(resolve_offsets(&blocks), [0.0, 0.0]);
    }

    /// Blocks at *different* anchor heights with a partial overlap. Every other
    /// case here shares an anchor y, where pushing up and pushing down happen to
    /// be numerically identical — so only this one pins the direction.
    #[test]
    fn a_partially_overlapping_block_is_pushed_by_exactly_the_overlap() {
        let blocks = [
            block(100.0, 100.0, 40.0, 12.0, 0), // occupies y 88..100
            block(100.0, 95.0, 40.0, 12.0, 1),  // occupies y 83..95, overlapping by 7
        ];
        let offsets = resolve_offsets(&blocks);
        assert_eq!(offsets[0], 0.0);
        assert_eq!(offsets[1], 7.0 + ft::SPEECH_GAP_PX);
    }

    /// A taller block (more queued lines) must be cleared by its real height.
    #[test]
    fn the_push_uses_the_blockers_measured_height() {
        let blocks = [
            block(100.0, 100.0, 40.0, 36.0, 0),
            block(100.0, 100.0, 40.0, 12.0, 1),
        ];
        let offsets = resolve_offsets(&blocks);
        assert_eq!(offsets[1], 36.0 + ft::SPEECH_GAP_PX);
    }

    use crate::game_ui::{GameUiAssets, GameViewport};
    use crate::network::events::ShowFloatingText;

    use crate::agent::AgentId;

    /// The agent every observer test speaks and bleeds through, standing on
    /// `SPEAKER_TILE`.
    const SPEAKER: AgentId = 1;

    fn speaker_tile() -> Position {
        Position::new(10, 10, 7)
    }

    /// Registers an agent standing on `pos`, exactly as `on_spawn_agent` does, so
    /// the observer can resolve the id the wire message carries back to a tile.
    fn agent_at(world: &mut World, agent_id: AgentId, pos: Position) -> Entity {
        let entity = world.spawn(pos).id();
        world.resource_mut::<Map>().add_agent(agent_id, entity);
        entity
    }

    /// A world with the three things the observer needs from the app: a viewport
    /// to parent to, the UI font, and a `Map` that can turn the agent id on the
    /// wire into the tile the text pins to.
    fn observer_world() -> World {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<Map>();
        world.insert_resource(GameUiAssets {
            font: Handle::default(),
            window: Default::default(),
            inventory: Default::default(),
            background_dark: Handle::default(),
            background_light: Handle::default(),
            bar_overlay: Handle::default(),
            title_background: Handle::default(),
        });
        world.spawn(GameViewport);
        world.add_observer(on_floating_text);
        agent_at(&mut world, SPEAKER, speaker_tile());
        world
    }

    fn hp_texts(world: &mut World) -> Vec<(String, f32)> {
        world
            .query::<(&Text, &FloatingText)>()
            .iter(world)
            .map(|(text, ft)| (text.0.clone(), ft.offset_y))
            .collect()
    }

    #[test]
    fn an_arriving_number_spawns_a_parented_child_of_the_viewport() {
        let mut world = observer_world();
        world.trigger(ShowFloatingText {
            text: "-25".to_owned(),
            agent_id: SPEAKER,
            text_type: FloatingTextType::HitPoints,
            color: None,
        });
        world.flush();

        let spawned = hp_texts(&mut world);
        assert_eq!(spawned, [("-25".to_owned(), 0.0)]);

        let viewport = world
            .query_filtered::<Entity, With<GameViewport>>()
            .single(&world)
            .unwrap();
        let child = world
            .query_filtered::<&ChildOf, With<FloatingText>>()
            .single(&world)
            .unwrap();
        assert_eq!(child.parent(), viewport, "must clip with the game viewport");
    }

    /// The deferred-reveal contract that placement depends on: a text cannot be
    /// centred on its anchor until it has been measured, and measurement is a frame
    /// behind by construction. Both kinds must therefore spawn hidden and carrying
    /// `Unplaced`; dropping either would place the first frame un-centred with
    /// nothing to catch it.
    #[test]
    fn both_kinds_spawn_hidden_and_unplaced() {
        for kind in [FloatingTextType::HitPoints, FloatingTextType::PlayerMessage] {
            let mut world = observer_world();
            world.trigger(ShowFloatingText {
                text: "-25".to_owned(),
                agent_id: SPEAKER,
                text_type: kind,
                color: None,
            });
            world.flush();

            let mut q = world.query_filtered::<&Visibility, (With<FloatingText>, With<Unplaced>)>();
            let visibility = q
                .single(&world)
                .expect("the spawned text must carry Unplaced");
            assert!(
                matches!(visibility, Visibility::Hidden),
                "{kind:?} must spawn hidden, got {visibility:?}"
            );
        }
    }

    #[test]
    fn two_mergeable_numbers_leave_one_entity_showing_the_sum() {
        let mut world = observer_world();
        for text in ["-12", "-8"] {
            world.trigger(ShowFloatingText {
                text: text.to_owned(),
                agent_id: SPEAKER,
                text_type: FloatingTextType::HitPoints,
                color: Some((255, 255, 255)),
            });
            world.flush();
        }

        assert_eq!(hp_texts(&mut world), [("-20".to_owned(), 0.0)]);
    }

    #[test]
    fn a_second_message_on_a_tile_queues_into_the_block() {
        let mut world = observer_world();
        for text in ["hi there", "how are you"] {
            world.trigger(ShowFloatingText {
                text: text.to_owned(),
                agent_id: SPEAKER,
                text_type: FloatingTextType::PlayerMessage,
                color: None,
            });
            world.flush();
        }

        let mut q = world.query::<&SpeechBlock>();
        let block = q.single(&world).unwrap();
        assert_eq!(block.lines.len(), 2, "one block, two lines");
        assert_eq!(block.compose(), "hi there\nhow are you");
    }

    #[test]
    fn a_message_on_another_tile_starts_its_own_block() {
        let mut world = observer_world();
        const NEIGHBOUR: AgentId = 2;
        agent_at(&mut world, NEIGHBOUR, Position::new(11, 10, 7));

        for (speaker, text) in [(SPEAKER, "hi"), (NEIGHBOUR, "hello")] {
            world.trigger(ShowFloatingText {
                text: text.to_owned(),
                agent_id: speaker,
                text_type: FloatingTextType::PlayerMessage,
                color: None,
            });
            world.flush();
        }

        assert_eq!(world.query::<&SpeechBlock>().iter(&world).count(), 2);
    }

    /// The wire message names an agent, but the text is pinned to a *tile*: two
    /// agents sharing one tile share one block, exactly as two messages from one
    /// agent do. Keying the block on the speaker instead would give this two
    /// blocks stacked on the same spot.
    #[test]
    fn two_agents_on_one_tile_share_a_block() {
        let mut world = observer_world();
        const NEIGHBOUR: AgentId = 2;
        agent_at(&mut world, NEIGHBOUR, speaker_tile());

        for (speaker, text) in [(SPEAKER, "hi"), (NEIGHBOUR, "hello")] {
            world.trigger(ShowFloatingText {
                text: text.to_owned(),
                agent_id: speaker,
                text_type: FloatingTextType::PlayerMessage,
                color: None,
            });
            world.flush();
        }

        let mut q = world.query::<&SpeechBlock>();
        let block = q.single(&world).expect("one tile, one block");
        assert_eq!(block.compose(), "hi\nhello");
    }

    /// The agent's position is read once, at spawn, and never again — so a speaker
    /// who walks away leaves the text behind on the tile they spoke from, and
    /// their next line starts a new block on the new tile.
    #[test]
    fn a_speaker_who_walks_away_leaves_the_text_behind() {
        let mut world = observer_world();
        let speaker = world.resource::<Map>().get_agent(SPEAKER).unwrap();

        let speak = |world: &mut World| {
            world.trigger(ShowFloatingText {
                text: "hi".to_owned(),
                agent_id: SPEAKER,
                text_type: FloatingTextType::PlayerMessage,
                color: None,
            });
            world.flush();
        };

        speak(&mut world);
        let walked_to = Position::new(speaker_tile().x + 1, speaker_tile().y, speaker_tile().z);
        world.entity_mut(speaker).insert(walked_to.clone());
        speak(&mut world);

        let anchors: Vec<Position> = world
            .query_filtered::<&FloatingText, With<SpeechBlock>>()
            .iter(&world)
            .map(|ft| ft.anchor.clone())
            .collect();
        assert_eq!(anchors.len(), 2, "the first block stayed where it was said");
        assert!(anchors.contains(&speaker_tile()));
        assert!(anchors.contains(&walked_to));
    }

    /// An agent the client has never seen — or has already despawned — has no
    /// tile to pin to, so its text is dropped rather than landing on some
    /// default tile.
    #[test]
    fn text_from_an_unknown_agent_is_dropped() {
        let mut world = observer_world();
        world.trigger(ShowFloatingText {
            text: "-25".to_owned(),
            agent_id: SPEAKER + 99,
            text_type: FloatingTextType::HitPoints,
            color: None,
        });
        world.flush();

        assert_eq!(world.query::<&FloatingText>().iter(&world).count(), 0);
    }

    #[test]
    fn the_oldest_line_drops_at_the_cap() {
        let mut world = observer_world();
        for i in 0..=ft::SPEECH_MAX_LINES {
            world.trigger(ShowFloatingText {
                text: format!("line {i}"),
                agent_id: SPEAKER,
                text_type: FloatingTextType::PlayerMessage,
                color: None,
            });
            world.flush();
        }

        let mut q = world.query::<&SpeechBlock>();
        let block = q.single(&world).unwrap();
        assert_eq!(block.lines.len(), ft::SPEECH_MAX_LINES);
        assert!(
            !block.compose().contains("line 0"),
            "the first line must have been evicted, got {:?}",
            block.compose()
        );
        assert!(block.compose().contains("line 5"));
    }

    use bevy::ecs::system::RunSystemOnce;

    fn advance(world: &mut World, ms: u64) {
        let mut time = world.resource_mut::<Time>();
        time.advance_by(Duration::from_millis(ms));
    }

    #[test]
    fn a_number_despawns_when_its_timer_finishes() {
        let mut world = observer_world();
        world.trigger(ShowFloatingText {
            text: "-1".to_owned(),
            agent_id: SPEAKER,
            text_type: FloatingTextType::HitPoints,
            color: None,
        });
        world.flush();

        advance(&mut world, ft::HP_DURATION_MS + 1);
        world.run_system_once(tick_hit_points).unwrap();

        assert_eq!(world.query::<&HitPointsText>().iter(&world).count(), 0);
    }

    #[test]
    fn a_number_fades_over_its_tail() {
        let mut world = observer_world();
        world.trigger(ShowFloatingText {
            text: "-1".to_owned(),
            agent_id: SPEAKER,
            text_type: FloatingTextType::HitPoints,
            color: None,
        });
        world.flush();

        // 95% through: past HP_FADE_START, not yet expired.
        advance(&mut world, (ft::HP_DURATION_MS as f32 * 0.95) as u64);
        world.run_system_once(tick_hit_points).unwrap();

        let mut q = world.query::<&TextColor>();
        let color = q.single(&world).unwrap();
        let a = color.0.alpha();
        assert!(a > 0.0 && a < 1.0, "expected a partial fade, got {a}");
    }

    #[test]
    fn an_expired_line_leaves_the_block_and_the_rest_stay() {
        let mut world = observer_world();
        // A short line, then a long one that outlives it.
        world.trigger(ShowFloatingText {
            text: "hi".to_owned(),
            agent_id: SPEAKER,
            text_type: FloatingTextType::PlayerMessage,
            color: None,
        });
        world.flush();
        world.trigger(ShowFloatingText {
            text: "x".repeat(200),
            agent_id: SPEAKER,
            text_type: FloatingTextType::PlayerMessage,
            color: None,
        });
        world.flush();

        advance(&mut world, ft::SPEECH_MIN_MS + 1);
        world.run_system_once(tick_speech_blocks).unwrap();

        let mut q = world.query::<&SpeechBlock>();
        let block = q.single(&world).unwrap();
        assert_eq!(block.lines.len(), 1, "the short line expired");
        assert!(block.compose().starts_with('x'));
    }

    /// A merge must not restart the absorbing number's timer, or a sustained
    /// stream of hits produces an immortal number. The comment on the merge arm
    /// says so; this is what stops a later refactor "fixing" it.
    #[test]
    fn merging_does_not_extend_the_numbers_life() {
        let mut world = observer_world();
        let hit = |world: &mut World| {
            world.trigger(ShowFloatingText {
                text: "-1".to_owned(),
                agent_id: SPEAKER,
                text_type: FloatingTextType::HitPoints,
                color: Some((255, 255, 255)),
            });
            world.flush();
        };

        hit(&mut world);
        advance(&mut world, 300);
        world.run_system_once(tick_hit_points).unwrap();
        hit(&mut world); // inside the merge window, so this merges
        advance(&mut world, 701); // 1001 ms since the *first* hit
        world.run_system_once(tick_hit_points).unwrap();

        assert_eq!(
            world.query::<&HitPointsText>().iter(&world).count(),
            0,
            "the merged number must still die on the original timer"
        );
    }

    #[test]
    fn a_block_despawns_when_its_last_line_expires() {
        let mut world = observer_world();
        world.trigger(ShowFloatingText {
            text: "hi".to_owned(),
            agent_id: SPEAKER,
            text_type: FloatingTextType::PlayerMessage,
            color: None,
        });
        world.flush();

        advance(&mut world, ft::SPEECH_MAX_MS + 1);
        world.run_system_once(tick_speech_blocks).unwrap();

        assert_eq!(world.query::<&SpeechBlock>().iter(&world).count(), 0);
    }

    /// The composed text must be rewritten only when a line actually left. A later
    /// system gates its work on `Changed<Text>`, so an unconditional write here
    /// would make it re-resolve every frame for nothing.
    #[test]
    fn ticking_without_an_expiry_does_not_touch_the_text() {
        let mut world = observer_world();
        world.trigger(ShowFloatingText {
            text: "hi".to_owned(),
            agent_id: SPEAKER,
            text_type: FloatingTextType::PlayerMessage,
            color: None,
        });
        world.flush();
        world.clear_trackers();

        advance(&mut world, 100);
        world.run_system_once(tick_speech_blocks).unwrap();

        let mut q = world.query_filtered::<Entity, Changed<Text>>();
        assert_eq!(
            q.iter(&world).count(),
            0,
            "no line expired, so Text must not have been rewritten"
        );
    }

    /// With the camera sitting exactly on the anchor's tile, a `HitPoints` anchor
    /// lands dead centre of the viewport — the identity case for the whole
    /// world→viewport conversion.
    #[test]
    fn an_anchor_under_the_camera_lands_at_the_viewport_centre() {
        let anchor = Position::new(100, 100, 7);
        let cam = tile_centre(&anchor);
        let size = Vec2::new(480.0, 352.0);

        let px = anchor_px(&anchor, FloatingTextType::HitPoints, cam, size);

        assert_eq!(px, size * 0.5);
    }

    /// The bug this pins: `Position::to_world` is the tile's top-left *corner*,
    /// so anchoring to it put every floating text half a tile up and to the left.
    /// With the camera on the corner, the anchor must land half a tile down and to
    /// the right of the viewport centre.
    #[test]
    fn the_anchor_is_the_tile_centre_not_its_corner() {
        use crate::conf::map::TILE_SIZE;
        let anchor = Position::new(100, 100, 7);
        let corner = anchor.to_world().truncate();
        let size = Vec2::new(480.0, 352.0);

        let px = anchor_px(&anchor, FloatingTextType::HitPoints, corner, size);

        assert_eq!(px.x, size.x * (TILE_SIZE / 2.0 / GAME_VIEW_WIDTH + 0.5));
        assert_eq!(px.y, size.y * (0.5 + TILE_SIZE / 2.0 / GAME_VIEW_HEIGHT));
    }

    /// Both other `anchor_px` tests put the camera exactly on the anchor, which
    /// zeroes `world - cam_pos` on both axes and multiplies the view divisors away.
    /// These two offset the camera by one tile so `GAME_VIEW_WIDTH` and
    /// `GAME_VIEW_HEIGHT` are actually load-bearing — they differ (480 vs 352), so
    /// swapping them moves text everywhere except dead centre.
    #[test]
    fn a_horizontal_camera_offset_divides_by_the_view_width() {
        use crate::conf::map::TILE_SIZE;
        let anchor = Position::new(100, 100, 7);
        let cam = tile_centre(&anchor) - Vec2::new(TILE_SIZE, 0.0);
        let size = Vec2::new(480.0, 352.0);

        let px = anchor_px(&anchor, FloatingTextType::HitPoints, cam, size);

        assert_eq!(px.x, size.x * (TILE_SIZE / GAME_VIEW_WIDTH + 0.5));
        assert_eq!(px.y, size.y * 0.5);
    }

    #[test]
    fn a_vertical_camera_offset_divides_by_the_view_height() {
        use crate::conf::map::TILE_SIZE;
        let anchor = Position::new(100, 100, 7);
        let cam = tile_centre(&anchor) - Vec2::new(0.0, TILE_SIZE);
        let size = Vec2::new(480.0, 352.0);

        let px = anchor_px(&anchor, FloatingTextType::HitPoints, cam, size);

        assert_eq!(px.x, size.x * 0.5);
        assert_eq!(px.y, size.y * (0.5 - TILE_SIZE / GAME_VIEW_HEIGHT));
    }

    /// The property that is invisible at a 1:1 viewport and wrong at every other
    /// size. The speech head offset is world-space, so it must scale with the view:
    /// double the viewport, double the on-screen gap. An offset applied in text
    /// space would produce the same gap at both sizes and drift off the sprite's
    /// head as the window grows.
    #[test]
    fn the_speech_head_offset_scales_with_the_viewport() {
        let anchor = Position::new(100, 100, 7);
        let cam = tile_centre(&anchor);
        let small = Vec2::new(480.0, 352.0);
        let large = small * 2.0;

        let gap = |size: Vec2| {
            anchor_px(&anchor, FloatingTextType::HitPoints, cam, size).y
                - anchor_px(&anchor, FloatingTextType::PlayerMessage, cam, size).y
        };

        assert!(gap(small) > 0.0, "speech must sit above the tile centre");
        assert_eq!(
            gap(large),
            gap(small) * 2.0,
            "a world-space offset scales with the view"
        );
    }

    // --- `position_floating_texts` / `resolve_speech_collisions` world tests ---
    //
    // Mutation testing found that both systems can be deleted wholesale with the
    // rest of the suite green: nothing above exercises `Node.left`/`Node.top`,
    // the rise animation, the collision offset, the floor filter, the
    // measured-size guard, or the `Unplaced` reveal handshake. These tests close
    // that hole at the `World` level, since `ComputedNode` is engine-written and
    // cannot be produced by triggering the observer alone.

    /// A world with a viewport, a game camera sitting exactly on `anchor`, and a
    /// player standing on `anchor`'s tile but at `player_z`. Mirrors
    /// `observer_world`, but for the two placement systems instead of the spawn
    /// observer.
    fn placement_world(anchor: &Position, player_z: u8) -> World {
        let mut world = World::new();
        world.init_resource::<Time>();

        world.spawn((
            GameViewport,
            ComputedNode {
                size: Vec2::new(480.0, 352.0),
                inverse_scale_factor: 1.0,
                ..Default::default()
            },
        ));
        world.spawn((
            GameCamera,
            GlobalTransform::from_translation(tile_centre(anchor).extend(0.0)),
        ));
        world.spawn((
            Player { agent_id: 1 },
            Position::new(anchor.x, anchor.y, player_z),
        ));

        world
    }

    fn fresh_hp_timer() -> Timer {
        Timer::new(Duration::from_millis(ft::HP_DURATION_MS), TimerMode::Once)
    }

    /// The viewport centre for a camera sitting exactly on the anchor's tile —
    /// the same identity case `an_anchor_under_the_camera_lands_at_the_viewport_centre`
    /// pins for `anchor_px` itself.
    const VIEWPORT_CENTRE: Vec2 = Vec2::new(240.0, 176.0);

    #[test]
    fn a_measured_number_is_centred_on_its_anchor() {
        let anchor = Position::new(100, 100, 7);
        let mut world = placement_world(&anchor, 7);

        let node_size = Vec2::new(20.0, 10.0);
        let entity = world
            .spawn((
                FloatingText {
                    kind: FloatingTextType::HitPoints,
                    anchor: anchor.clone(),
                    spawned_at: Duration::ZERO,
                    offset_y: 0.0,
                },
                HitPointsText {
                    value: Some(5),
                    color: Color::WHITE,
                    timer: fresh_hp_timer(),
                },
                ComputedNode {
                    size: node_size,
                    inverse_scale_factor: 1.0,
                    ..Default::default()
                },
                Node::default(),
                Visibility::Hidden,
                Unplaced,
            ))
            .id();

        // First run: the entity is measured, so its position is written this
        // frame, and the queued `remove::<Unplaced>()` command is applied by
        // `run_system_once` before it returns. But `want` was computed from the
        // query's snapshot at the *start* of this run, when `Unplaced` was still
        // present — so visibility does not flip to `Visible` until the *next*
        // run sees it gone. This is the same one-frame handshake the module
        // docs describe for `ComputedNode`, just for `Unplaced` instead.
        world.run_system_once(position_floating_texts).unwrap();

        // VIEWPORT_CENTRE - node_size / 2, and the bottom edge sitting on the
        // anchor: 240 - 10 = 230, 176 - 10 = 166. No rise, no offset.
        let node = world.get::<Node>(entity).unwrap();
        assert_eq!(node.left, Val::Px(230.0));
        assert_eq!(node.top, Val::Px(166.0));
        assert!(
            world.get::<Unplaced>(entity).is_none(),
            "a measured HitPointsText must have Unplaced removed"
        );
        assert_eq!(
            *world.get::<Visibility>(entity).unwrap(),
            Visibility::Hidden,
            "Unplaced's removal is deferred, so visibility cannot flip within the same run"
        );

        // Second run: Unplaced is gone, so the text is now visible.
        world.run_system_once(position_floating_texts).unwrap();
        assert_eq!(
            *world.get::<Visibility>(entity).unwrap(),
            Visibility::Visible
        );
        let node = world.get::<Node>(entity).unwrap();
        assert_eq!(node.left, Val::Px(230.0));
        assert_eq!(node.top, Val::Px(166.0));
    }

    /// Would catch the `- rise` term being dropped from the top calculation: at
    /// fraction 0 and fraction 0.5 the tops differ by exactly `risen(0.5)`.
    #[test]
    fn a_number_rises_as_its_timer_advances() {
        let anchor = Position::new(100, 100, 7);
        let node_size = Vec2::new(20.0, 10.0);

        let spawn = |world: &mut World, fraction_ms: u64| {
            let mut timer = fresh_hp_timer();
            timer.tick(Duration::from_millis(fraction_ms));
            world
                .spawn((
                    FloatingText {
                        kind: FloatingTextType::HitPoints,
                        anchor: anchor.clone(),
                        spawned_at: Duration::ZERO,
                        offset_y: 0.0,
                    },
                    HitPointsText {
                        value: Some(5),
                        color: Color::WHITE,
                        timer,
                    },
                    ComputedNode {
                        size: node_size,
                        inverse_scale_factor: 1.0,
                        ..Default::default()
                    },
                    Node::default(),
                    Visibility::Hidden,
                    Unplaced,
                ))
                .id()
        };

        let mut world_at_rest = placement_world(&anchor, 7);
        let at_rest = spawn(&mut world_at_rest, 0);
        world_at_rest
            .run_system_once(position_floating_texts)
            .unwrap();
        let top_at_rest = world_at_rest.get::<Node>(at_rest).unwrap().top;

        let mut world_risen = placement_world(&anchor, 7);
        // Half the duration: fraction 0.5.
        let risen_entity = spawn(&mut world_risen, ft::HP_DURATION_MS / 2);
        world_risen
            .run_system_once(position_floating_texts)
            .unwrap();
        let top_risen = world_risen.get::<Node>(risen_entity).unwrap().top;

        let (Val::Px(rest), Val::Px(risen_px)) = (top_at_rest, top_risen) else {
            panic!("expected Val::Px on both");
        };
        assert_eq!(
            rest - risen_px,
            risen(0.5),
            "top must move up by exactly the rise at fraction 0.5"
        );
        assert_eq!(risen(0.5), ft::HP_RISE_PX / 2.0);
    }

    #[test]
    fn a_text_on_another_floor_stays_hidden() {
        let anchor = Position::new(100, 100, 7);
        // Player stands one floor below the anchor's tile.
        let mut world = placement_world(&anchor, 8);

        let entity = world
            .spawn((
                FloatingText {
                    kind: FloatingTextType::HitPoints,
                    anchor: anchor.clone(),
                    spawned_at: Duration::ZERO,
                    offset_y: 0.0,
                },
                HitPointsText {
                    value: Some(5),
                    color: Color::WHITE,
                    timer: fresh_hp_timer(),
                },
                ComputedNode {
                    size: Vec2::new(20.0, 10.0),
                    inverse_scale_factor: 1.0,
                    ..Default::default()
                },
                Node::default(),
                Visibility::Hidden,
            ))
            .id();

        world.run_system_once(position_floating_texts).unwrap();

        assert_eq!(
            *world.get::<Visibility>(entity).unwrap(),
            Visibility::Hidden,
            "a text on another floor must stay hidden even though it is measured"
        );
    }

    #[test]
    fn an_unmeasured_text_stays_hidden_and_unplaced() {
        let anchor = Position::new(100, 100, 7);
        let mut world = placement_world(&anchor, 7);

        let zero_size = || ComputedNode {
            size: Vec2::ZERO,
            inverse_scale_factor: 1.0,
            ..Default::default()
        };

        // The natural case: freshly spawned, never measured, still carrying
        // `Unplaced`.
        let fresh = world
            .spawn((
                FloatingText {
                    kind: FloatingTextType::HitPoints,
                    anchor: anchor.clone(),
                    spawned_at: Duration::ZERO,
                    offset_y: 0.0,
                },
                HitPointsText {
                    value: Some(5),
                    color: Color::WHITE,
                    timer: fresh_hp_timer(),
                },
                zero_size(),
                Node::default(),
                Visibility::Hidden,
                Unplaced,
            ))
            .id();

        // A second, already-placed text (no `Unplaced`) whose node happens to
        // report zero size this frame. With `Unplaced` present, `fresh` above
        // would stay hidden from the `unplaced.is_none()` clause alone even if
        // the `measured` clause were dropped — so it cannot by itself pin
        // `measured` in the `want` calculation. This entity has no `Unplaced` to
        // fall back on, so only the `measured` clause can keep it hidden.
        let already_placed = world
            .spawn((
                FloatingText {
                    kind: FloatingTextType::HitPoints,
                    anchor: anchor.clone(),
                    spawned_at: Duration::ZERO,
                    offset_y: 0.0,
                },
                HitPointsText {
                    value: Some(5),
                    color: Color::WHITE,
                    timer: fresh_hp_timer(),
                },
                zero_size(),
                Node::default(),
                Visibility::Hidden,
            ))
            .id();

        world.run_system_once(position_floating_texts).unwrap();

        assert_eq!(
            *world.get::<Visibility>(fresh).unwrap(),
            Visibility::Hidden,
            "an unmeasured text must stay hidden"
        );
        assert!(
            world.get::<Unplaced>(fresh).is_some(),
            "an unmeasured text must keep Unplaced, since it was never placed"
        );
        assert_eq!(
            *world.get::<Visibility>(already_placed).unwrap(),
            Visibility::Hidden,
            "measured=false must hide the text regardless of Unplaced state"
        );
    }

    /// Would catch the `- text.offset_y` term being dropped from the top
    /// calculation: two otherwise-identical numbers differing only in
    /// `offset_y` must land exactly `offset_y` apart.
    #[test]
    fn a_collision_offset_reaches_the_node_position() {
        let anchor = Position::new(100, 100, 7);
        let node_size = Vec2::new(20.0, 10.0);

        let spawn = |world: &mut World, offset_y: f32| {
            world
                .spawn((
                    FloatingText {
                        kind: FloatingTextType::HitPoints,
                        anchor: anchor.clone(),
                        spawned_at: Duration::ZERO,
                        offset_y,
                    },
                    HitPointsText {
                        value: Some(5),
                        color: Color::WHITE,
                        timer: fresh_hp_timer(),
                    },
                    ComputedNode {
                        size: node_size,
                        inverse_scale_factor: 1.0,
                        ..Default::default()
                    },
                    Node::default(),
                    Visibility::Hidden,
                    Unplaced,
                ))
                .id()
        };

        let mut world_flat = placement_world(&anchor, 7);
        let flat = spawn(&mut world_flat, 0.0);
        world_flat.run_system_once(position_floating_texts).unwrap();
        let top_flat = world_flat.get::<Node>(flat).unwrap().top;

        let mut world_pushed = placement_world(&anchor, 7);
        let pushed = spawn(&mut world_pushed, 14.0);
        world_pushed
            .run_system_once(position_floating_texts)
            .unwrap();
        let top_pushed = world_pushed.get::<Node>(pushed).unwrap().top;

        let (Val::Px(flat_px), Val::Px(pushed_px)) = (top_flat, top_pushed) else {
            panic!("expected Val::Px on both");
        };
        assert_eq!(
            flat_px - pushed_px,
            14.0,
            "a 14 px collision offset must move the node exactly 14 px higher"
        );
    }

    /// `VIEWPORT_CENTRE` is the identity case documented above: sanity-check it
    /// against `anchor_px` itself so the constant cannot silently drift from the
    /// function it stands in for.
    #[test]
    fn viewport_centre_matches_anchor_px_at_the_identity_case() {
        let anchor = Position::new(100, 100, 7);
        let cam = tile_centre(&anchor);
        let size = Vec2::new(480.0, 352.0);
        assert_eq!(
            anchor_px(&anchor, FloatingTextType::HitPoints, cam, size),
            VIEWPORT_CENTRE
        );
    }
}
