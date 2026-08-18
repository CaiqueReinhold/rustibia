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
}
