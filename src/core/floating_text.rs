//! World-anchored transient text: damage numbers and speech over a tile.
//!
//! Distinct from `core::text`, which anchors to the *viewport*. Everything here
//! is pinned to a tile and outlives whatever caused it.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatingTextType {
    HitPoints,
    PlayerMessage,
}
