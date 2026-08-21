use std::sync::Arc;
use std::time::Duration;

use bevy::mesh::MeshTag;
use bevy::prelude::*;

use crate::conf::missiles::FLIGHT_MS_PER_ROOT_TILE;
use crate::conf::z_order::MISSILE_Z_OFFSET;
use crate::core::effects::{
    EffectInstance, EffectMaterial, EffectMaterials, anchor, init_instance, init_material,
};
use crate::core::{Appearances, InstanceManager};
use crate::map::{FloorEntities, Position};
use crate::network::events::LaunchMissile;

/// The pattern cell a missile draws, which is its flight direction.
///
/// All 56 missiles are 3x3, and the grid is a compass rose laid out spatially:
/// NW N NE across the top, W . E across the middle, SW S SE across the bottom.
///
/// OT derives the direction as an angle rather than by comparing signs: eight
/// 45-degree sectors with East centred on 0. The `-dy` is what makes "north"
/// mean DECREASING tile y -- up the screen -- and dropping it flips every
/// missile vertically.
fn missile_cell(from: &Position, to: &Position) -> (u32, u32) {
    let dx = to.x as f32 - from.x as f32;
    let dy = to.y as f32 - from.y as f32;

    let mut degrees = (-dy).atan2(dx).to_degrees();
    if degrees < 0.0 {
        degrees += 360.0;
    }

    // Offsetting by half a sector before dividing turns "nearest of eight" into
    // a floor, and the `% 8` folds 337.5..360 back onto East at 0.
    match ((degrees + 22.5) / 45.0) as u32 % 8 {
        0 => (2, 1), // East
        1 => (2, 0), // NorthEast
        2 => (1, 0), // North
        3 => (0, 0), // NorthWest
        4 => (0, 1), // West
        5 => (0, 2), // SouthWest
        6 => (1, 2), // South
        _ => (2, 2), // SouthEast
    }
}

/// How long the flight takes: 150 ms times the SQUARE ROOT of the tile
/// distance. See `conf::missiles::FLIGHT_MS_PER_ROOT_TILE` for why the root is
/// there and what it does to the feel.
fn flight_duration(from: &Position, to: &Position) -> Duration {
    let dx = to.x as f32 - from.x as f32;
    let dy = to.y as f32 - from.y as f32;
    let tiles = dx.hypot(dy);

    Duration::from_millis((FLIGHT_MS_PER_ROOT_TILE * tiles.sqrt()) as u64)
}

/// Where the missile sits at `fraction` of its flight.
///
/// Takes tiles rather than world coordinates so the `to_world` conversion and
/// the z offset live here instead of being the caller's to remember. Lerping
/// the whole `Vec3` gets the world-space y flip and the floor offset for free
/// -- and lerping z is what keeps the missile above the tile it is currently
/// crossing, rather than sliding behind items further down and right.
fn missile_position(from: &Position, to: &Position, fraction: f32, sprite_size: Vec2) -> Vec3 {
    let world = from
        .to_world()
        .lerp(to.to_world(), fraction.clamp(0.0, 1.0));

    anchor(world, sprite_size, MISSILE_Z_OFFSET)
}

/// A missile in flight. Its instance slot is written once at launch -- the
/// sprite is static and the direction cannot change mid-flight -- so only the
/// `Transform` moves.
#[derive(Component)]
pub struct Missile {
    from: Position,
    to: Position,
    sprite_size: Vec2,
    elapsed: Duration,
    duration: Duration,
}

pub fn on_launch_missile(
    event: On<LaunchMissile>,
    mut commands: Commands,
    mut instances: ResMut<InstanceManager<EffectInstance>>,
    mut effect_materials: ResMut<EffectMaterials>,
    mut materials: ResMut<Assets<EffectMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    appearances: Res<Appearances>,
    floors: Res<FloorEntities>,
) {
    // A missile that goes nowhere has no direction and a zero duration, which
    // would divide by zero in `fly_missiles`. OT schedules immediate removal;
    // never spawning is the same outcome without the round trip, and it makes
    // the division unreachable rather than merely guarded.
    if event.from == event.to {
        return;
    }

    // `None` means the server named a missile these assets do not have. A
    // missing projectile is cosmetic, so warn and skip rather than ending the
    // session as the outfit path does.
    let Some(sprite) = appearances.get_missile(event.missile_id) else {
        warn!(
            "server sent missile {}, which this client\'s assets do not have",
            event.missile_id
        );
        return;
    };
    let sprite = Arc::clone(sprite);
    let sheet = appearances.get_sheet(&sprite.group);
    let sprite_size = sheet.sprite_size;

    if !effect_materials.by_group.contains_key(&sprite.group) {
        init_material(
            &sprite.group,
            sheet,
            &mut materials,
            &mut meshes,
            &mut effect_materials,
        );
    }
    let (mesh, material) = effect_materials.by_group[&sprite.group].clone();

    // `cell_*` and not `pattern_*`: `sprite.pattern_x` is the GRID\'S WIDTH (3),
    // while this is a coordinate WITHIN that grid. Reusing one name for both is
    // how the index formula below gets written wrong.
    let (cell_x, cell_y) = missile_cell(&event.from, &event.to);

    let index = instances.alloc_index();
    let instance = instances.get_mut(index);
    init_instance(instance, &sprite, sprite_size, cell_x);
    // The sprite id, resolved once. `resolve_simple_sprite_ids`\' formula with
    // phase 0, one layer and no z pattern collapses to
    // `cell_y * grid_width + cell_x`.
    instance.sprite_id = sprite
        .sprite_ids
        .get((cell_y * sprite.pattern_x + cell_x) as usize)
        .copied()
        .unwrap_or(0);

    let entity = commands
        .spawn((
            Missile {
                from: event.from.clone(),
                to: event.to.clone(),
                sprite_size,
                elapsed: Duration::ZERO,
                duration: flight_duration(&event.from, &event.to),
            },
            Mesh2d(mesh),
            MeshMaterial2d(material),
            MeshTag(index),
            Transform::from_translation(missile_position(&event.from, &event.to, 0.0, sprite_size)),
            Visibility::Inherited,
        ))
        .id();
    // Parented to the source floor. The wire carries a z on both positions, but
    // combat only ever produces same-floor shots.
    commands
        .entity(floors.floors[event.from.z as usize])
        .add_child(entity);
}

/// Advances every missile and collects the ones that have arrived.
pub fn fly_missiles(
    mut commands: Commands,
    time: Res<Time>,
    mut missiles: Query<(Entity, &mut Missile, &mut Transform)>,
) {
    for (entity, mut missile, mut transform) in &mut missiles {
        missile.elapsed += time.delta();
        if missile.elapsed >= missile.duration {
            commands.entity(entity).despawn();
            continue;
        }

        let fraction = missile.elapsed.as_secs_f32() / missile.duration.as_secs_f32();
        transform.translation =
            missile_position(&missile.from, &missile.to, fraction, missile.sprite_size);
    }
}

pub fn on_remove_missile(
    event: On<Remove, Missile>,
    tags: Query<&MeshTag, With<Missile>>,
    mut instances: ResMut<InstanceManager<EffectInstance>>,
) {
    let Ok(tag) = tags.get(event.entity) else {
        return;
    };

    instances.dealloc_index(tag.0);
}

/// Despawns missiles still in flight when the session ends.
///
/// Mandatory for the same reason effects need it: missiles are children of the
/// floor entities, which are `Startup`-spawned and outlive the session, so
/// nothing else collects them. The instance manager is NOT reset here --
/// `effects::cleanup_session` owns that, and both share the one buffer.
pub(super) fn cleanup_session(mut commands: Commands, missiles: Query<Entity, With<Missile>>) {
    for entity in &missiles {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 3x3 grid is a compass rose, and each of these four catches a
    /// different way of getting it wrong.
    #[test]
    fn a_missiles_cell_is_its_flight_direction() {
        let origin = Position::new(100, 100, 7);

        // North is DECREASING tile y -- up the screen. North and South are the
        // only pair that catches a dropped y negation, and that mistake flips
        // every missile in the game while looking fine in a single-case test.
        assert_eq!(missile_cell(&origin, &Position::new(100, 96, 7)), (1, 0));
        assert_eq!(missile_cell(&origin, &Position::new(100, 104, 7)), (1, 2));
        // East catches an x/y axis swap.
        assert_eq!(missile_cell(&origin, &Position::new(104, 100, 7)), (2, 1));
        // A diagonal catches a sector-boundary error the cardinals sail past.
        assert_eq!(missile_cell(&origin, &Position::new(104, 96, 7)), (2, 0));
    }

    /// Four tiles, not one, and the distance is the whole point of the test: a
    /// 1-tile shot is 150 ms with OR without the square root, so it cannot
    /// detect a missing one. Four tiles gives 300 ms against 600 ms.
    #[test]
    fn a_flight_takes_the_root_of_its_distance() {
        let duration = flight_duration(&Position::new(100, 100, 7), &Position::new(104, 100, 7));

        assert_eq!(duration, Duration::from_millis(300));
    }

    /// The endpoints are hard-coded rather than compared against `anchor`,
    /// which `missile_position` itself calls -- that would assert `f(x) == f(x)`
    /// and pass against any implementation.
    ///
    /// Tile (100, 100, 7) is world (3200, -3200); a 32 px sprite is nudged to
    /// (3216, -3216). Tile (104, 100, 7) is four tiles east, so 128 px right.
    #[test]
    fn a_missile_starts_on_its_source_tile_and_ends_on_its_target() {
        use crate::conf::z_order::TOP_Z_OFFSET;

        let from = Position::new(100, 100, 7);
        let to = Position::new(104, 100, 7);
        let size = Vec2::new(32.0, 32.0);

        let start = missile_position(&from, &to, 0.0, size);
        let end = missile_position(&from, &to, 1.0, size);

        assert_eq!(start.x, 3216.0);
        assert_eq!(start.y, -3216.0);
        assert_eq!(end.x, 3344.0);
        assert_eq!(end.y, -3216.0);

        // Above whatever the tile it is over holds, at both ends.
        assert!(start.z > from.to_world().z + TOP_Z_OFFSET);
        assert!(end.z > to.to_world().z + TOP_Z_OFFSET);
    }
}
