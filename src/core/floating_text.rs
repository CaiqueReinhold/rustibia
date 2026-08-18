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
//! offsets (rise, collision push, line gap) are **logical px** applied after that
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

// Nothing outside this file's tests uses these yet; the systems that consume them
// land over the following tasks. The suppression comes off once the last one is
// wired up, which is what proves everything here is actually reachable.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;

use crate::conf::floating_text as ft;
use crate::conf::ui::chat::LOCAL_CHANNEL_COLOR;
use crate::map::Position;

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
}
