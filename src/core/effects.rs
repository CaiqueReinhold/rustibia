use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};

use crate::conf::effects::STATIC_DURATION;
use crate::conf::z_order::EFFECT_Z_OFFSET;
use crate::core::sprite::{AnimationLoop, SpriteAnimation, SpriteConfig};
use crate::core::{Appearances, InstanceManager, SpriteAnimator};
use crate::map::FloorEntities;
use crate::map::Position;
use crate::network::events::ShowEffect;

/// One effect's slot in the shader storage buffer.
///
/// Byte-identical to `ItemInstance`: both describe a single-layer atlas sprite
/// with a bounding box and a shift, which is all `shaders/items.wgsl` consumes.
#[repr(C)]
#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectInstance {
    pub sprite_id: u32,
    pub _pad: u32, // required std430 alignment padding before vec2
    pub bbox_min: Vec2,
    pub bbox_size: Vec2,
    pub shift: Vec2,
}

/// Effects reuse `shaders/items.wgsl` rather than copying it: that shader is
/// generic over "single-layer atlas sprite", and is named after its first caller
/// rather than its only possible one.
///
/// A separate material type, instead of importing `ItemMaterial`, keeps the
/// effect system independent of `ItemsPlugin` — and avoids a second
/// `Material2dPlugin` registration for the same type, which panics.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct EffectMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,

    #[uniform(2)]
    pub atlas_grid: Vec2,

    #[storage(3, read_only)]
    pub instances: Handle<ShaderStorageBuffer>,

    #[uniform(4)]
    pub mesh_size: Vec2,
}

impl Material2d for EffectMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/items.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/items.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// One buffer for every effect on screen, and one mesh and material per atlas
/// sheet group, built on first use. There are seven groups.
#[derive(Resource, Debug, Default)]
pub struct EffectMaterials {
    by_group: HashMap<String, (Handle<Mesh>, Handle<EffectMaterial>)>,
    buffer: Handle<ShaderStorageBuffer>,
}

pub fn setup_resources(mut commands: Commands, mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    commands.insert_resource(EffectMaterials {
        by_group: HashMap::new(),
        buffer: buffers.add(ShaderStorageBuffer::new(&[0], RenderAssetUsages::all())),
    });
}

/// A live effect. Children of the floor entity for their tile, so this
/// module only has to know which floor to parent to — not how floor
/// occlusion decides what to hide.
#[derive(Component)]
pub struct Effect {
    /// `None` when the animation ends itself — the animator is the authority.
    /// `Some` for the 16 effects whose loop mode never would.
    ttl: Option<Timer>,
}

fn init_material(
    group: &str,
    appearances: &Appearances,
    materials: &mut Assets<EffectMaterial>,
    meshes: &mut Assets<Mesh>,
    effect_materials: &mut EffectMaterials,
) {
    let sheet = appearances.get_sheet(group);
    let material = materials.add(EffectMaterial {
        texture: sheet.texture().clone(),
        atlas_grid: sheet.grid_size,
        mesh_size: sheet.sprite_size,
        instances: effect_materials.buffer.clone(),
    });
    let mesh = meshes.add(Mesh::from(Rectangle::new(
        sheet.sprite_size.x,
        sheet.sprite_size.y,
    )));
    effect_materials
        .by_group
        .insert(group.to_string(), (mesh, material));
}

/// Fills in everything about an instance except its sprite id, which needs an
/// animator that does not exist yet — the same split `items::instancing`'s
/// `init_instance` makes.
///
/// `boxes` is indexed by `pattern_x` alone: effect 41 is 2x2 with 2 boxes,
/// effect 1 has 6 phases and 1 box.
fn init_instance(
    instance: &mut EffectInstance,
    sprite: &SpriteConfig,
    sprite_size: Vec2,
    pattern_x: u32,
) {
    instance.shift = sprite.shift;
    // A box's `.max` holds a SIZE, not a maximum corner — `read_sprite_config`
    // parses `[x, y, w, h]` into `Rect { min, max }` and the shader consumes
    // the second pair as an extent. Do not "correct" this to `max - min`; it
    // shrinks every effect.
    match sprite.boxes.get(pattern_x as usize) {
        Some(bbox) => {
            instance.bbox_min = bbox.min;
            instance.bbox_size = bbox.max;
        }
        None => {
            instance.bbox_min = Vec2::ZERO;
            instance.bbox_size = sprite_size;
        }
    }
}

pub fn on_show_effect(
    event: On<ShowEffect>,
    mut commands: Commands,
    mut instances: ResMut<InstanceManager<EffectInstance>>,
    mut effect_materials: ResMut<EffectMaterials>,
    mut materials: ResMut<Assets<EffectMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    appearances: Res<Appearances>,
    floors: Res<FloorEntities>,
) {
    // `None` means the server named an effect these assets do not have. Unlike
    // `get_outfit`'s equivalent gap, which triggers `ClientOutdated` and ends
    // the session, a missing effect is cosmetic: warn and skip this cast
    // rather than tearing down the connection over a hit spark the client
    // failed to draw.
    let Some(sprite) = appearances.get_effect(event.effect_id) else {
        warn!(
            "server sent effect {}, which this client's assets do not have",
            event.effect_id
        );
        return;
    };
    let sprite = Arc::clone(sprite);

    if !effect_materials.by_group.contains_key(&sprite.group) {
        init_material(
            &sprite.group,
            &appearances,
            &mut materials,
            &mut meshes,
            &mut effect_materials,
        );
    }
    let (mesh, material) = effect_materials.by_group[&sprite.group].clone();
    // `init_material` just read the sheet to build this material, and stashed
    // its sprite size in `mesh_size` — reading it back off the material avoids
    // asking `Appearances` for the sheet a second time.
    let sprite_size = materials.get(&material).unwrap().mesh_size;

    // Computed once per message, not once per tile: `pass_duration` samples a
    // `NonUniform` animation's phases with `fastrand`, so rolling it inside the
    // loop below would let one area effect's tiles die at independently
    // sampled moments instead of all together.
    let ttl = lifetime(&sprite.animation);

    for tile in effect_tiles(&event.position, &event.delta) {
        let (pattern_x, pattern_y) = pattern_for(&tile, &sprite);

        let index = instances.alloc_index();
        let instance = instances.get_mut(index);
        init_instance(instance, &sprite, sprite_size, pattern_x);

        let animator = SpriteAnimator::new(Arc::clone(&sprite), pattern_x, pattern_y, 0);
        instance.sprite_id = animator.current_sprite_ids[0];

        let entity = commands
            .spawn((
                Effect {
                    ttl: ttl.map(|d| Timer::new(d, TimerMode::Once)),
                },
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                MeshTag(index),
                Transform::from_translation(anchor(tile.to_world(), sprite_size)),
                Visibility::Inherited,
                animator,
            ))
            .id();
        commands
            .entity(floors.floors[tile.z as usize])
            .add_child(entity);
    }
}

/// Every tile the message paints: the base position, then one per delta.
///
/// The wire carries no floor per tile, so an area effect is flat by
/// construction.
fn effect_tiles(base: &Position, delta: &[(i8, i8)]) -> Vec<Position> {
    let mut tiles = Vec::with_capacity(delta.len() + 1);
    tiles.push(base.clone());
    tiles.extend(
        delta
            .iter()
            .map(|(dx, dy)| base.delta(*dx as i32, *dy as i32)),
    );
    tiles
}

/// The pattern a tile draws, taken from its own absolute coordinates.
///
/// 196 of the 207 effects are 1x1 and always get `(0, 0)`. For the eleven 2x2
/// and 3x3 ones this is what tiles a field seamlessly across an area instead of
/// repeating one corner — and it is stable per tile, so a repeat of the same
/// area effect does not shimmer.
fn pattern_for(position: &Position, sprite: &SpriteConfig) -> (u32, u32) {
    (
        position.x as u32 % sprite.pattern_x.max(1),
        position.y as u32 % sprite.pattern_y.max(1),
    )
}

/// Where an effect's quad sits, from its tile's world position and its sheet's
/// sprite size.
///
/// `Position::to_world` returns the tile's TOP-LEFT CORNER. A 64 px quad centres
/// on that corner correctly — large Tibia sprites extend up and to the left of
/// their tile — and a 32 px one has to be nudged half a tile down and right. The
/// rule is per axis because two effect sheet groups are 32x64 and 64x32.
fn anchor(world: Vec3, sprite_size: Vec2) -> Vec3 {
    let half_tile_x = if sprite_size.x <= 32.0 { 16.0 } else { 0.0 };
    let half_tile_y = if sprite_size.y <= 32.0 { -16.0 } else { 0.0 };
    Vec3::new(
        world.x + half_tile_x,
        world.y + half_tile_y,
        world.z + EFFECT_Z_OFFSET,
    )
}

/// How long an effect lives, or `None` when its own animation ends it.
///
/// `Counted` is 191 of the 207 effects and is left to `SpriteAnimator`, because
/// `count` is a number of RUNS: effect 77 (`loop_count: 402`) is `NonUniform`
/// with 16 phases summing to 1490 ms, so its real lifetime is
/// `402 * 1490 ms ≈ 599 s`, about ten minutes — a rule expressed as a single
/// pass would cut it to 1490 ms, a 400x truncation. The other 16 never finish
/// on their own.
///
/// The consequence: a counted effect holds its entity and instance slot for
/// its entire run, up to ~10 minutes for effect 77, because this function
/// defers to the animator instead of capping it. Unreachable today — the
/// server only ever sends effects 1, 3 and 4 — but a real consequence of the
/// design, not a hypothetical one, if that ever changes.
///
/// The `never_advances` arm comes first because `SpriteAnimation::Static`
/// reports `AnimationLoop::Infinite` — matching on the loop mode alone would
/// give a static effect a zero-length pass.
///
/// `PingPong` is handed one pass exactly like `Infinite`, but "one pass" here
/// means one traversal of the phases, not a full there-and-back cycle:
/// `SpriteAnimator::settle_on_timed_phase`'s doc records that cycle as
/// `2n - 2`, so a real ping-pong effect would be despawned mid-return. No
/// shipped effect is PINGPONG, so this arm is a placeholder for data that
/// does not exist, not a considered lifetime.
fn lifetime(animation: &SpriteAnimation) -> Option<Duration> {
    if animation.never_advances() {
        return Some(STATIC_DURATION);
    }
    match animation.loop_mode() {
        AnimationLoop::Counted { .. } => None,
        AnimationLoop::Infinite | AnimationLoop::PingPong => Some(animation.pass_duration()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sprite::{AnimationLoop, SpriteAnimation, SpriteConfig};
    use std::time::Duration;

    fn config(pattern_x: u32, pattern_y: u32) -> SpriteConfig {
        SpriteConfig {
            id: 1,
            group: "effect-32-32-0".to_string(),
            pattern_x,
            pattern_y,
            pattern_z: 1,
            layers: 1,
            sprite_ids: vec![0],
            animation: SpriteAnimation::Static,
            boxes: Vec::new(),
            shift: Vec2::ZERO,
        }
    }

    /// An empty delta is what the server sends today, and it must mean "just
    /// this tile" rather than "no tiles".
    #[test]
    fn an_empty_delta_paints_only_the_base_tile() {
        let tiles = effect_tiles(&Position::new(100, 200, 7), &[]);

        assert_eq!(tiles, vec![Position::new(100, 200, 7)]);
    }

    /// Deltas are relative to the base tile, and the base tile plays too. The
    /// wire carries no floor per tile, so an area effect is flat.
    #[test]
    fn deltas_paint_extra_tiles_around_the_base() {
        let tiles = effect_tiles(&Position::new(100, 200, 7), &[(1, 0), (0, -1), (-1, -1)]);

        assert_eq!(
            tiles,
            vec![
                Position::new(100, 200, 7),
                Position::new(101, 200, 7),
                Position::new(100, 199, 7),
                Position::new(99, 199, 7),
            ]
        );
    }

    /// 196 of the 207 effects are 1x1 and must always land on pattern zero.
    #[test]
    fn a_single_pattern_effect_always_draws_pattern_zero() {
        let sprite = config(1, 1);

        assert_eq!(pattern_for(&Position::new(100, 200, 7), &sprite), (0, 0));
        assert_eq!(pattern_for(&Position::new(101, 201, 7), &sprite), (0, 0));
    }

    /// The eleven patterned effects tile across an area by absolute coordinate,
    /// which is what makes a 3x3 field seamless instead of nine copies of one
    /// corner.
    #[test]
    fn a_patterned_effect_walks_its_patterns_across_adjacent_tiles() {
        let sprite = config(3, 3);

        let row: Vec<(u32, u32)> = (99..102)
            .map(|x| pattern_for(&Position::new(x, 200, 7), &sprite))
            .collect();

        assert_eq!(row, vec![(0, 2), (1, 2), (2, 2)]);
    }

    /// Stability matters: a repeat of the same area effect must not shimmer.
    /// `1028 % 3 == 2` and `1029 % 3 == 0`, so the same tile always gets `(2, 0)`
    /// — not just "the same answer as itself", which every deterministic
    /// function trivially gives.
    #[test]
    fn a_tiles_pattern_does_not_change_between_casts() {
        let sprite = config(3, 3);
        let position = Position::new(1028, 1029, 7);

        assert_eq!(pattern_for(&position, &sprite), (2, 0));
    }

    /// `pattern_x` and `pattern_y` are indistinguishable on a square pattern —
    /// every other case here uses `config(1, 1)` or `config(3, 3)`. A 2x4
    /// pattern catches a divisor swapped onto the wrong axis: `5 % 2 = 1` and
    /// `7 % 4 = 3`, so a swap would report `(1, 1)` instead.
    #[test]
    fn a_rectangular_pattern_uses_the_matching_divisor_per_axis() {
        let sprite = config(2, 4);

        assert_eq!(pattern_for(&Position::new(5, 7, 0), &sprite), (1, 3));
    }

    /// `read_sprite_config` never validates `pattern_x`/`pattern_y`, so a zero
    /// dimension is not provably impossible — and without `.max(1)` it would
    /// panic on a modulo by zero rather than degrade to pattern zero.
    #[test]
    fn a_zero_pattern_dimension_does_not_panic() {
        let sprite = config(0, 0);

        assert_eq!(pattern_for(&Position::new(5, 7, 0), &sprite), (0, 0));
    }

    /// `to_world` returns the tile's TOP-LEFT CORNER. A 32 px quad is centred on
    /// its transform, so it must be nudged half a tile down and right to cover
    /// the tile it belongs to.
    #[test]
    fn a_32px_sprite_is_nudged_onto_its_tile() {
        let placed = anchor(Vec3::new(100.0, 200.0, 5.0), Vec2::new(32.0, 32.0));

        assert_eq!(placed.x, 116.0);
        assert_eq!(placed.y, 184.0);
    }

    /// A 64 px sprite centres on the corner correctly — large Tibia sprites
    /// extend up and to the left of the tile they occupy.
    #[test]
    fn a_64px_sprite_keeps_the_tile_corner() {
        let placed = anchor(Vec3::new(100.0, 200.0, 5.0), Vec2::new(64.0, 64.0));

        assert_eq!(placed.x, 100.0);
        assert_eq!(placed.y, 200.0);
    }

    /// Two effect sheet groups are 32x64 and 64x32, so the correction has to be
    /// decided per axis. Applying it to both, or neither, puts them half a tile
    /// out on one axis.
    #[test]
    fn a_mixed_size_sprite_is_corrected_on_one_axis_only() {
        let placed = anchor(Vec3::new(100.0, 200.0, 5.0), Vec2::new(32.0, 64.0));

        assert_eq!(placed.x, 116.0, "32 px wide, so nudged");
        assert_eq!(placed.y, 200.0, "64 px tall, so not");
    }

    /// A box's `.max` holds a SIZE, not a maximum corner — `read_sprite_config`
    /// parses `[x, y, w, h]` into `Rect { min, max }` and the shader consumes
    /// the second pair as an extent. This is the test that catches a `max -
    /// min` "correction": with `min = (4, 6)` and `max = (24, 20)`, that
    /// mutation would report `(20, 14)` instead of `(24, 20)`.
    #[test]
    fn init_instance_with_a_box_uses_its_max_as_the_size_verbatim() {
        let mut sprite = config(1, 1);
        sprite.boxes = vec![Rect {
            min: Vec2::new(4.0, 6.0),
            max: Vec2::new(24.0, 20.0),
        }];
        sprite.shift = Vec2::new(1.0, 2.0);
        let mut instance = EffectInstance::default();

        init_instance(&mut instance, &sprite, Vec2::new(32.0, 32.0), 0);

        assert_eq!(instance.bbox_min, Vec2::new(4.0, 6.0));
        assert_eq!(instance.bbox_size, Vec2::new(24.0, 20.0));
        assert_eq!(instance.shift, Vec2::new(1.0, 2.0));
    }

    /// A sheet group's sprite size is the fallback, not a hardcoded 32x32 —
    /// unlike `items::instancing::init_instance`, which does hardcode it and
    /// would clip a boxless effect on a 64x64 sheet.
    #[test]
    fn init_instance_without_a_box_falls_back_to_the_sheets_sprite_size() {
        let sprite = config(1, 1); // boxes: Vec::new()
        let mut instance = EffectInstance::default();

        init_instance(&mut instance, &sprite, Vec2::new(64.0, 64.0), 0);

        assert_eq!(instance.bbox_min, Vec2::ZERO);
        assert_eq!(instance.bbox_size, Vec2::new(64.0, 64.0));
    }

    /// Effects draw over creatures and under top items.
    #[test]
    fn an_effect_sits_between_the_agent_and_the_top_item_planes() {
        use crate::conf::z_order::{AGENT_Z_OFFSET, TOP_Z_OFFSET};

        let placed = anchor(Vec3::new(0.0, 0.0, 5.0), Vec2::new(32.0, 32.0));

        assert!(placed.z > 5.0 + AGENT_Z_OFFSET);
        assert!(placed.z < 5.0 + TOP_Z_OFFSET);
    }

    /// 191 of the 207 effects are counted, and `count` is a number of RUNS, not
    /// phases: effect 77 runs 402 times over a 1490 ms pass, about ten minutes
    /// total. Only the animator knows a pass's real length, so it stays the
    /// authority — the entity and its instance slot are held for that whole
    /// run, unreachable today because the server only ever sends effects 1, 3
    /// and 4.
    #[test]
    fn a_counted_effect_defers_to_its_animator() {
        let animation = SpriteAnimation::Uniform {
            loop_mode: AnimationLoop::Counted { count: 402 },
            phase_count: 2,
            phase_duration: Duration::from_millis(4),
        };

        assert_eq!(lifetime(&animation), None);
    }

    /// The 13 infinite effects would otherwise never be despawned.
    #[test]
    fn an_endless_effect_lives_exactly_one_pass() {
        let infinite = SpriteAnimation::Uniform {
            loop_mode: AnimationLoop::Infinite,
            phase_count: 8,
            phase_duration: Duration::from_millis(100),
        };
        let pingpong = SpriteAnimation::Uniform {
            loop_mode: AnimationLoop::PingPong,
            phase_count: 4,
            phase_duration: Duration::from_millis(50),
        };

        assert_eq!(lifetime(&infinite), Some(Duration::from_millis(800)));
        assert_eq!(lifetime(&pingpong), Some(Duration::from_millis(200)));
    }

    /// Effects 200, 211 and 212 have no animation at all. `loop_mode()` reports
    /// `Infinite` for a static animation, so a lifetime rule written on the loop
    /// mode alone would give them a zero-length pass and flash them for one
    /// frame.
    #[test]
    fn a_static_effect_gets_the_fixed_duration() {
        assert_eq!(lifetime(&SpriteAnimation::Static), Some(STATIC_DURATION));
    }
}
