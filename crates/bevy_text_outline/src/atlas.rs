use bevy::asset::AssetId;
use bevy::asset::RenderAssetUsages;
use bevy::ecs::component::Component;
use bevy::image::{DynamicTextureAtlasBuilder, Image};
use bevy::math::{Rect, UVec2};
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, FromWorld, Handle, Resource, TextureAtlasLayout, World};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Cache key for a rasterized outline glyph.
/// Encodes: (fill atlas image ID, glyph index in that atlas, physical outline width bits).
pub type OutlineCacheKey = (AssetId<Image>, usize, u32);

/// Per-glyph outline atlas location, parallel to [`TextLayoutInfo::glyphs`].
#[derive(Clone, Debug)]
pub struct PerGlyphOutlineInfo {
    /// UV rect of this glyph's outline in the outline atlas.
    pub rect: Rect,
    /// Asset ID of the outline atlas image.
    pub image_id: AssetId<Image>,
}

/// Per-entity cache of outline glyph positions, parallel to [`TextLayoutInfo::glyphs`].
///
/// `None` entries correspond to glyphs that don't produce an outline (e.g. whitespace).
#[derive(Component, Default)]
pub struct OutlineGlyphAtlasInfos {
    pub infos: Vec<Option<PerGlyphOutlineInfo>>,
}

const ATLAS_SIZE: u32 = 1024;

/// Manages the texture atlas that stores pre-rasterized outline glyphs.
#[derive(Resource)]
pub struct OutlineAtlas {
    /// Packs glyph images into the atlas texture.
    pub builder: DynamicTextureAtlasBuilder,
    /// Handle to the atlas layout (glyph rects).
    pub layout_handle: Handle<TextureAtlasLayout>,
    /// Handle to the atlas image (pixel data).
    pub image_handle: Handle<Image>,
    /// Maps outline cache key → the rect in the atlas layout.
    pub cache: HashMap<OutlineCacheKey, Rect>,
}

impl FromWorld for OutlineAtlas {
    fn from_world(world: &mut World) -> Self {
        let atlas_image = Image::new_fill(
            Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            // MAIN_WORLD so DynamicTextureAtlasBuilder can write to it;
            // RENDER_WORLD so the GPU can sample it.
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );

        let image_handle = world.resource_mut::<Assets<Image>>().add(atlas_image);

        let layout_handle = world
            .resource_mut::<Assets<TextureAtlasLayout>>()
            .add(TextureAtlasLayout::new_empty(UVec2::splat(ATLAS_SIZE)));

        OutlineAtlas {
            builder: DynamicTextureAtlasBuilder::new(UVec2::splat(ATLAS_SIZE), 1),
            layout_handle,
            image_handle,
            cache: HashMap::default(),
        }
    }
}
