use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::Extract;
use bevy::render::sync_world::TemporaryRenderEntity;
use bevy::text::TextLayoutInfo;
use bevy::ui::{CalculatedClip, ComputedNode, ComputedUiTargetCamera};
use bevy::ui_render::{
    ExtractedGlyph, ExtractedUiItem, ExtractedUiNode, ExtractedUiNodes, UiCameraMap,
    stack_z_offsets,
};

use crate::atlas::OutlineGlyphAtlasInfos;
use crate::component::TextOutline;

/// Injects outline glyphs into the UI render pipeline for each UI `Text` entity that has a
/// [`TextOutline`] component and a prepared [`OutlineGlyphAtlasInfos`].
///
/// Must run before `extract_text_sections` in [`ExtractSchedule`] so outline glyphs are
/// pushed to [`ExtractedUiNodes`] at a lower z-order than the fill, ensuring outlines
/// render behind the fill text.
pub fn extract_outline_glyphs(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    camera_map: Extract<UiCameraMap>,
    query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &TextLayoutInfo,
            &TextOutline,
            &OutlineGlyphAtlasInfos,
        )>,
    >,
) {
    let ExtractedUiNodes {
        glyphs, uinodes, ..
    } = &mut *extracted_uinodes;
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        uinode,
        global_transform,
        inherited_visibility,
        clip,
        target,
        layout_info,
        textoutline,
        glyph_infos,
    ) in query.iter()
    {
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(target) else {
            continue;
        };

        // Same transform formula as extract_text_sections.
        let transform =
            Affine2::from(*global_transform) * Affine2::from_translation(-0.5 * uinode.size());

        let color: LinearRgba = textoutline.color.into();

        let mut start = glyphs.len();
        let mut batch_len = 0usize;
        let mut current_image_id = None;

        for (glyph, outline_info) in layout_info.glyphs.iter().zip(glyph_infos.infos.iter()) {
            let Some(info) = outline_info else { continue };

            // Flush when atlas image changes (shouldn't normally happen for our single atlas).
            if current_image_id.is_some_and(|id| id != info.image_id) && batch_len > 0 {
                uinodes.push(ExtractedUiNode {
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    // Slightly below TEXT so outlines render behind fill.
                    z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT - 0.01,
                    image: current_image_id.unwrap(),
                    clip: clip.map(|c| c.clip),
                    extracted_camera_entity,
                    transform,
                    item: ExtractedUiItem::Glyphs {
                        range: start..(start + batch_len),
                    },
                    main_entity: entity.into(),
                });
                start += batch_len;
                batch_len = 0;
            }
            current_image_id = Some(info.image_id);

            // In the UI pipeline, glyph.position is the CENTER of the fill quad.
            // Our outline bitmap is dilated symmetrically, so its center also aligns
            // with glyph.position — no offset correction needed.
            glyphs.push(ExtractedGlyph {
                color,
                translation: glyph.position,
                rect: info.rect,
            });
            batch_len += 1;
        }

        // Flush final batch.
        if batch_len > 0 {
            uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT - 0.01,
                image: current_image_id.unwrap(),
                clip: clip.map(|c| c.clip),
                extracted_camera_entity,
                transform,
                item: ExtractedUiItem::Glyphs {
                    range: start..(start + batch_len),
                },
                main_entity: entity.into(),
            });
        }
    }
}
