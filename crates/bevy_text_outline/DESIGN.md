# bevy_text_outline — Design Document

## Context

This crate adds stroke outlines to Bevy `Text` UI nodes. It is used in this project to render
actor display names (health-coloured names above player/NPC heads) with a black outline for
legibility against the game background.

---

## Why not `Text2d`?

The first implementation rendered names as `Text2d` child entities of each actor. This approach
was abandoned for two separate reasons:

### 1. Faded outlines from the 2× render texture

The game view is rendered into an off-screen image at 2× the logical viewport size and
downsampled to screen via `ImageSampler::linear()`. Any `Text2d` that lives in game-world space
is captured in this render pass, so its glyph and outline pixels are blended with adjacent
pixels on downscale. The result is a soft, faded outline instead of a crisp 1-pixel stroke.

Moving text to the UI camera (which renders directly to the screen at native resolution) avoids
the downsampling step entirely.

### 2. Sprite pass renders before UI nodes

Even when a `Text2d` entity is placed on the UI camera (`RenderLayers::layer(1)`), it still
goes through the **sprite render pass**, which runs before the **UI render pass**. The UI
side panel is an ordinary UI node and therefore renders on top of `Text2d` sprites. Names were
visible for a single frame on startup (before the UI loaded) and then permanently hidden behind
the panel.

Switching to UI `Text` nodes makes names part of the UI render pass, so they composite above
all UI nodes at the correct z-order.

---

## Architecture

```
Main world (PostUpdate)                 Render world (ExtractSchedule)
┌─────────────────────────┐             ┌────────────────────────────────────────┐
│ prepare_outline_glyphs  │             │ extract_outline_glyphs                 │
│                         │             │   (before extract_text_sections)       │
│ For each entity with     │  ──sync──▶ │                                        │
│ Text + TextOutline:      │             │ Reads OutlineGlyphAtlasInfos           │
│  • Read fill glyph alpha │             │ Writes ExtractedUiNodes with           │
│    from Bevy font atlas  │             │   z_order = stack_index + TEXT - 0.01  │
│  • Circular dilation     │             │   (renders behind fill text)           │
│  • Pack into OutlineAtlas│             └────────────────────────────────────────┘
│  • Write OutlineGlyphAtlasInfos        │
└─────────────────────────┘
```

### Components and resources

| Symbol | Kind | Purpose |
|---|---|---|
| `TextOutline` | Component | User-facing: outline `width` (logical px) + `color` |
| `OutlineGlyphAtlasInfos` | Component | Per-entity, parallel to `TextLayoutInfo::glyphs`; each entry is the outline rect in the atlas + atlas image ID |
| `OutlineAtlas` | Resource | Shared 1024×1024 `Rgba8UnormSrgb` texture atlas that stores all rasterised outline glyphs. Keyed by `(fill_atlas_id, glyph_index, width_bits)` |
| `OutlineCacheKey` | Type alias | `(AssetId<Image>, usize, u32)` — uniquely identifies one outline shape |

---

## Outline rasterisation (`prepare.rs`)

Traditional outline approaches (swash stroke, SDF expand) produce outlines in a different
coordinate space than the fill glyph, causing sub-pixel misalignment that is visible at small
font sizes.

This crate uses **morphological dilation on the fill glyph's own pixel data**:

1. Read the fill glyph's alpha channel from Bevy's `Rgba8UnormSrgb` font atlas
   (`alpha = data[pixel_index * 4 + 3]`).
2. Dilate by `radius = round(outline_width × scale_factor)` pixels using a circular
   structuring element (max-pooling within a disk of that radius).
3. Output size is exactly `(fill_w + 2×radius) × (fill_h + 2×radius)`.
4. Pack into the `OutlineAtlas` via `DynamicTextureAtlasBuilder`.

Because the outline is derived directly from the fill pixels, it is **guaranteed to be
pixel-perfect with respect to the fill** — there is no path-to-pixel conversion uncertainty.

Glyphs are cached by `OutlineCacheKey` so each unique (glyph, width) combination is rasterised
once and reused across all entities and frames.

---

## Injection into the UI pipeline (`extract.rs`)

Bevy's UI text renderer works through `ExtractedUiNodes`. Each text section becomes an
`ExtractedUiNode` containing a range into the `glyphs` buffer. The same mechanism is used here:

```
ExtractedUiNodes {
    glyphs: Vec<ExtractedGlyph>,   // indexed by range in each uinode
    uinodes: Vec<ExtractedUiNode>, // one per batch (per-atlas-image)
}
```

Key details:

- **Transform**: identical to `extract_text_sections` —
  `Affine2::from(*global_transform) * Affine2::from_translation(-0.5 * uinode.size())`

- **Glyph position**: `glyph.position` in `TextLayoutInfo` is the **center** of the fill quad
  (this follows from `QUAD_VERTEX_POSITIONS = [(-0.5,…),(0.5,…),…]` in Bevy's UI renderer).
  The outline bitmap is dilated symmetrically, so its center also aligns with `glyph.position`.
  No extra offset is needed.

- **Z-order**: `stack_index as f32 + stack_z_offsets::TEXT - 0.01`
  This places outlines just below the fill text (`stack_z_offsets::TEXT = 0.06`) in the same
  UI stack, so fill always composites on top.

- **Scheduling**: `extract_outline_glyphs` runs `.before(extract_text_sections)` inside
  `RenderUiSystems::ExtractText`. Fill glyphs are pushed after outlines, so the GPU render
  order is outline → fill even within the same frame.

---

## Actor display name positioning (`src/actor/`)

Each actor spawns a UI `Hud` node (absolutely positioned, `ZIndex(100)`) that contains:
- A `DisplayName` child: `Text` + `TextOutline` + health-coloured text
- Optional `HudBar` children for health and mana

The `Hud` node is positioned each frame by `update_hud_positions` in `PostUpdate`, which
converts actor world-space coordinates to UI logical pixels:

```
uv.x = (world.x - cam.x) / GAME_VIEW_WIDTH  + 0.5
uv.y = 0.5 - (world.y - cam.y) / GAME_VIEW_HEIGHT  - y_offset / GAME_VIEW_HEIGHT

screen_px = top_left_of_viewport + uv * viewport_size_px
```

This is the inverse of the hover-detection math in `update_hover_state` (`interaction.rs`),
which converts screen → world. Both use `UiGlobalTransform.translation` as the viewport center
and `ComputedNode.size()` as the viewport extent in logical pixels.

The `Hud` node uses `UiTransform::from_translation(Val2::new(Val::Px(x), Val::Px(y)))` to
place its top-left corner at the computed screen position. Because the node uses
`align_items: Center` and grows downward (`FlexDirection::Column`), the name is
horizontally centred and the HUD bars stack below it.

---

## What was explicitly rejected

| Approach | Reason rejected |
|---|---|
| `Text2d` on the game camera | Downsampled through 2× render texture; faded outlines |
| `Text2d` on the UI camera | Still goes through sprite pass; renders behind UI nodes |
| Third dedicated "name overlay" camera | Extra complexity; user preference against it |
| swash stroke / SDF outline | Sub-pixel misalignment with fill at small font sizes |

---

## Bevy version

Bevy `0.18`. The crate uses internal rendering types (`ExtractedUiNodes`, `ExtractedGlyph`,
`ExtractedUiItem`, `UiCameraMap`, `stack_z_offsets`, `RenderUiSystems`) which are not part of
Bevy's stable public API and may change across minor versions.
