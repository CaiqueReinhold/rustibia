use bevy::asset::Assets;
use bevy::asset::RenderAssetUsages;
use bevy::ecs::query::Changed;
use bevy::ecs::system::{Commands, Query, ResMut};
use bevy::image::Image;
use bevy::math::Rect;
use bevy::prelude::TextureAtlasLayout;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::TextLayoutInfo;

use crate::atlas::{OutlineAtlas, OutlineGlyphAtlasInfos, PerGlyphOutlineInfo};
use crate::component::TextOutline;

/// Generates outline glyph textures by morphologically dilating the fill glyph pixels.
///
/// For each glyph in the entity's text layout:
/// 1. Reads the fill glyph's pixel data from Bevy's font atlas.
/// 2. Runs a circular dilation with `radius = round(physical_outline)` pixels.
/// 3. Packs the result into the shared [`OutlineAtlas`].
///
/// Because we operate directly on the fill glyph's rendered pixels, the outline bitmap
/// is always exactly `(fill_w + 2*radius, fill_h + 2*radius)` — no swash placement
/// uncertainty. The extract step adds the corresponding `+radius` offset so the outline
/// is perfectly centred on the fill.
pub fn prepare_outline_glyphs(
    mut commands: Commands,
    mut outline_atlas: ResMut<OutlineAtlas>,
    mut outline_textures: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut query: Query<
        (
            bevy::ecs::entity::Entity,
            &TextLayoutInfo,
            &TextOutline,
            Option<&mut OutlineGlyphAtlasInfos>,
        ),
        Changed<TextLayoutInfo>,
    >,
) {
    for (entity, layout_info, textoutline, maybe_infos) in query.iter_mut() {
        let physical_outline = textoutline.width * layout_info.scale_factor;
        let radius = physical_outline.round() as u32;
        if radius == 0 {
            let new_infos = OutlineGlyphAtlasInfos {
                infos: layout_info.glyphs.iter().map(|_| None).collect(),
            };
            if let Some(mut existing) = maybe_infos {
                *existing = new_infos;
            } else {
                commands.entity(entity).insert(new_infos);
            }
            continue;
        }

        let width_bits = physical_outline.to_bits();
        let mut infos: Vec<Option<PerGlyphOutlineInfo>> =
            Vec::with_capacity(layout_info.glyphs.len());

        for glyph in &layout_info.glyphs {
            let atlas_info = &glyph.atlas_info;
            let outline_cache_key = (
                atlas_info.texture,
                atlas_info.location.glyph_index,
                width_bits,
            );

            let rect = if let Some(&rect) = outline_atlas.cache.get(&outline_cache_key) {
                rect
            } else {
                // Get the fill glyph's rect in the fill atlas.
                let Some(fill_layout) = layouts.get(atlas_info.texture_atlas) else {
                    infos.push(None);
                    continue;
                };
                let fill_rect = fill_layout.textures[atlas_info.location.glyph_index];
                let fill_w = fill_rect.width();
                let fill_h = fill_rect.height();

                // Get the fill atlas image.
                let Some(fill_image) = outline_textures.get(atlas_info.texture) else {
                    infos.push(None);
                    continue;
                };
                let Some(fill_data) = fill_image.data.as_deref() else {
                    infos.push(None);
                    continue;
                };

                // Extract alpha channel from fill glyph region.
                // Fill atlas is Rgba8UnormSrgb: 4 bytes per pixel, alpha at byte 3.
                let atlas_w = fill_image.width();
                let fill_alpha = extract_alpha_rgba(
                    fill_data,
                    atlas_w,
                    fill_rect.min.x,
                    fill_rect.min.y,
                    fill_w,
                    fill_h,
                );

                // Dilate alpha by radius pixels (circular structuring element).
                let out_w = fill_w + radius * 2;
                let out_h = fill_h + radius * 2;
                let dilated = dilate_alpha(&fill_alpha, fill_w, fill_h, radius);

                // Build RGBA image: white pixels with dilated alpha.
                let rgba: Vec<u8> = dilated.iter().flat_map(|&a| [255u8, 255, 255, a]).collect();

                if out_w == 0 || out_h == 0 {
                    infos.push(None);
                    continue;
                }

                let glyph_image = Image::new(
                    Extent3d {
                        width: out_w,
                        height: out_h,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    rgba,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD,
                );

                let atlas_img = outline_textures
                    .get_mut(&outline_atlas.image_handle)
                    .unwrap();
                let layout = layouts.get_mut(&outline_atlas.layout_handle).unwrap();
                let Ok(idx) = outline_atlas
                    .builder
                    .add_texture(layout, &glyph_image, atlas_img)
                else {
                    infos.push(None);
                    continue;
                };

                let rect: Rect = layout.textures[idx].as_rect();
                outline_atlas.cache.insert(outline_cache_key, rect);
                rect
            };

            infos.push(Some(PerGlyphOutlineInfo {
                rect,
                image_id: outline_atlas.image_handle.id(),
            }));
        }

        let new_infos = OutlineGlyphAtlasInfos { infos };
        if let Some(mut existing) = maybe_infos {
            *existing = new_infos;
        } else {
            commands.entity(entity).insert(new_infos);
        }
    }
}

/// Extracts the alpha channel from a region of an `Rgba8UnormSrgb` image.
fn extract_alpha_rgba(data: &[u8], atlas_w: u32, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut alpha = Vec::with_capacity((w * h) as usize);
    for row in 0..h {
        for col in 0..w {
            let pixel_idx = ((y + row) * atlas_w + (x + col)) as usize;
            let byte_idx = pixel_idx * 4 + 3; // alpha is 4th byte
            alpha.push(data[byte_idx]);
        }
    }
    alpha
}

/// Circular morphological dilation: for each output pixel, the value is the max alpha
/// within a circle of `radius` pixels in the source.
///
/// Output size is `(w + 2*radius) × (h + 2*radius)`.
fn dilate_alpha(src: &[u8], w: u32, h: u32, radius: u32) -> Vec<u8> {
    let out_w = w + radius * 2;
    let out_h = h + radius * 2;
    let r = radius as i32;
    let r2 = r * r;

    let mut out = vec![0u8; (out_w * out_h) as usize];

    for oy in 0..out_h as i32 {
        for ox in 0..out_w as i32 {
            // Map output pixel back to source coords (offset by -radius).
            let sx_center = ox - r;
            let sy_center = oy - r;

            let mut max_val = 0u8;
            // Sample the neighbourhood in source space.
            let sx_min = (sx_center - r).max(0);
            let sx_max = (sx_center + r).min(w as i32 - 1);
            let sy_min = (sy_center - r).max(0);
            let sy_max = (sy_center + r).min(h as i32 - 1);

            'outer: for sy in sy_min..=sy_max {
                for sx in sx_min..=sx_max {
                    let dx = sx - sx_center;
                    let dy = sy - sy_center;
                    if dx * dx + dy * dy <= r2 {
                        let a = src[(sy * w as i32 + sx) as usize];
                        if a > max_val {
                            max_val = a;
                            if max_val == 255 {
                                break 'outer;
                            }
                        }
                    }
                }
            }

            out[(oy * out_w as i32 + ox) as usize] = max_val;
        }
    }

    out
}
