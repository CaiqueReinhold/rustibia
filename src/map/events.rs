use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    agent::{UpdateElevation, WalkingDirection},
    conf::map::{TILES_X, TILES_Y},
    core::ItemConfigs,
    items::{ChangedTileQueue, Item},
    map::{Map, Position, minimap::MinimapData},
    network::{
        ItemStack,
        events::{ClientOutdated, DescribeMap, TileChanged},
    },
};

/// Yields `(index, position)` pairs for a `DescribeMap` payload, where `index`
/// is the tile's slot in the message.
fn iter_viewport(pos: &Position, floor: u8) -> impl Iterator<Item = (usize, Position)> {
    let floor_offset = pos.z as i16 - floor as i16;
    let half_w = (TILES_X / 2) as i16;
    let half_h = (TILES_Y / 2) as i16;
    let x = pos.x as i16;
    let y = pos.y as i16;

    let x_start = (x - half_w + floor_offset).max(0) as u16;
    let x_end = (x + half_w + floor_offset).max(0) as u16;
    let y_start = (y - half_h + floor_offset).max(0) as u16;
    let y_end = (y + half_h + floor_offset).max(0) as u16;
    let z = floor;

    (0..TILES_Y).flat_map(move |row| {
        (0..TILES_X).filter_map(move |col| {
            let x = x_start + col as u16;
            let y = y_start + row as u16;
            (x <= x_end && y <= y_end).then_some((row * TILES_X + col, Position { x, y, z }))
        })
    })
}

fn iter_expansion(
    pos: &Position,
    direction: &WalkingDirection,
    floor: u8,
) -> Box<dyn Iterator<Item = Position>> {
    let floor_offset = pos.z as i16 - floor as i16;
    let half_w = (TILES_X / 2) as i16;
    let half_h = (TILES_Y / 2) as i16;
    let x = pos.x as i16;
    let y = pos.y as i16;
    let z = floor;

    let x_start = (x - half_w + floor_offset).max(0) as u16;
    let x_end = (x + half_w + floor_offset) as u16;
    let y_start = (y - half_h + floor_offset).max(0) as u16;
    let y_end = (y + half_h + floor_offset) as u16;

    let top_row = {
        (x_start..=x_end).map(move |xi| Position {
            x: xi,
            y: y_start,
            z,
        })
    };
    let bottom_row = (x_start..=x_end).map(move |xi| Position { x: xi, y: y_end, z });
    let left_col = {
        (y_start..=y_end).map(move |yi| Position {
            x: x_start,
            y: yi,
            z,
        })
    };
    let right_col = (y_start..=y_end).map(move |yi| Position { x: x_end, y: yi, z });

    match *direction {
        WalkingDirection::North => Box::new(top_row),
        WalkingDirection::South => Box::new(bottom_row),
        WalkingDirection::East => Box::new(right_col),
        WalkingDirection::West => Box::new(left_col),
        // For diagonals: full edge row + edge column excluding the shared corner.
        WalkingDirection::NorthEast => Box::new(top_row.chain(right_col.skip(1))),
        WalkingDirection::NorthWest => Box::new(top_row.chain(left_col.skip(1))),
        WalkingDirection::SouthEast => {
            Box::new(bottom_row.chain(right_col.take((y_end - y_start) as usize)))
        }
        WalkingDirection::SouthWest => {
            Box::new(bottom_row.chain(left_col.take((y_end - y_start) as usize)))
        }
    }
}

/// Returns `false` when the server named an item this client doesn't have; the
/// caller must stop and let the [`ClientOutdated`] trigger take over instead of
/// writing a half-resolved tile into the map.
#[must_use]
fn update_tile(
    tile: &ItemStack,
    position: &Position,
    map: &mut Map,
    config: &ItemConfigs,
    minimap: &mut MinimapData,
    commands: &mut Commands,
) -> bool {
    let mut items = Vec::with_capacity(8);
    for item in tile {
        let Some((item_id, amount)) = item else {
            break;
        };
        let Some(config) = config.items.get(item_id) else {
            commands.trigger(ClientOutdated);
            return false;
        };
        items.push(Arc::new(Item::new(config.clone(), *amount as u32)));
    }

    let old_elevation = map.get_elevation(position);
    map.replace_tile(items, position);
    let new_elevation = map.get_elevation(position);

    if old_elevation != new_elevation {
        commands.trigger(UpdateElevation {
            pos: position.clone(),
        });
    }

    let friction = if map.avoid(position) {
        0
    } else {
        map.get_tile_friction(position).unwrap_or(0)
    };

    minimap.update_tile(
        position,
        map.get_minimap_color(position).unwrap_or(0),
        friction,
    );

    true
}

pub(super) fn on_describe_map(
    event: On<DescribeMap>,
    mut commands: Commands,
    config: Res<ItemConfigs>,
    mut map: ResMut<Map>,
    mut queue: ResMut<ChangedTileQueue>,
    mut minimap: ResMut<MinimapData>,
) {
    for (i, position) in iter_viewport(&event.center, event.floor) {
        let tile = event.tiles[i];
        if !update_tile(
            &tile,
            &position,
            &mut map,
            &config,
            &mut minimap,
            &mut commands,
        ) {
            return;
        }
        queue.changed_positions.push_back(position);
    }
}

pub fn on_player_walk_ack(
    commands: &mut Commands,
    queue: &mut ChangedTileQueue,
    map: &mut Map,
    config: &ItemConfigs,
    minimap: &mut MinimapData,
    player_pos: &Position,
    direction: WalkingDirection,
    floor_tiles: &[(u8, Box<[ItemStack]>)],
) {
    for (floor, tiles) in floor_tiles {
        for (i, position) in iter_expansion(player_pos, &direction, *floor).enumerate() {
            // Both sides derive the expansion from the same clamped rectangle,
            // so the counts line up; this only guards against a server that
            // disagrees anyway. Stop at what arrived rather than indexing past it.
            let Some(tile) = tiles.get(i) else {
                break;
            };
            if !update_tile(tile, &position, map, config, minimap, commands) {
                return;
            }
            queue.changed_positions.push_back(position);
        }
    }
}

pub(super) fn on_tile_changed(
    event: On<TileChanged>,
    config: Res<ItemConfigs>,
    mut map: ResMut<Map>,
    mut queue: ResMut<ChangedTileQueue>,
    mut minimap: ResMut<MinimapData>,
    mut commands: Commands,
) {
    if !update_tile(
        &event.items,
        &event.position,
        &mut map,
        &config,
        &mut minimap,
        &mut commands,
    ) {
        return;
    }
    queue.changed_positions.push_back(event.position.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOOR: u8 = 7;

    fn center(x: u16, y: u16) -> Position {
        Position { x, y, z: FLOOR }
    }

    /// Away from the map border nothing is clamped: the payload is described in
    /// full, so index and iteration order run together.
    #[test]
    fn an_unclamped_viewport_covers_every_slot_in_order() {
        let tiles: Vec<(usize, Position)> = iter_viewport(&center(100, 100), FLOOR).collect();

        assert_eq!(tiles.len(), TILES_X * TILES_Y);
        for (i, (index, _)) in tiles.iter().enumerate() {
            assert_eq!(*index, i, "no gaps when nothing is clamped");
        }
        assert_eq!(tiles[0].1, center(100 - 9, 100 - 7), "top-left corner");
        assert_eq!(tiles.last().unwrap().1, center(100 + 9, 100 + 7));
    }

    /// The regression: against the west edge the server still writes rows
    /// `TILES_X` apart, so the second row has to resume at index `TILES_X` —
    /// not at the number of tiles the first row actually described.
    #[test]
    fn a_clamped_viewport_keeps_the_server_row_stride() {
        // x - 9 would be -6, so the described columns are 0..=12 — 13 of 19.
        let tiles: Vec<(usize, Position)> = iter_viewport(&center(3, 100), FLOOR).collect();
        let described_columns = 13;

        let first_row: Vec<_> = tiles.iter().take(described_columns).collect();
        assert_eq!(first_row[0].1, center(0, 100 - 7), "clamped to x = 0");
        assert_eq!(first_row[described_columns - 1].1, center(12, 100 - 7));

        let (index, position) = &tiles[described_columns];
        assert_eq!(*index, TILES_X, "the next row starts a full stride along");
        assert_eq!(*position, center(0, 100 - 6));

        assert_eq!(tiles.len(), described_columns * TILES_Y);
    }

    /// Both clamps at once, and the rows that fall off the top of the map are
    /// dropped whole rather than shifting everything below them.
    #[test]
    fn a_corner_viewport_drops_rows_and_columns_together() {
        let tiles: Vec<(usize, Position)> = iter_viewport(&center(3, 2), FLOOR).collect();

        assert_eq!(tiles[0].1, center(0, 0));
        for (index, position) in &tiles {
            assert_eq!(position.x, (index % TILES_X) as u16);
            assert_eq!(position.y, (index / TILES_X) as u16);
        }
    }

    /// A straight step describes one edge line; a diagonal describes that line
    /// plus the perpendicular one, minus the corner they share. Getting the
    /// count wrong leaves the last tile of the column stale.
    #[test]
    fn expansion_covers_one_edge_line_per_straight_step() {
        let center = center(100, 100);
        for direction in [
            WalkingDirection::North,
            WalkingDirection::South,
            WalkingDirection::East,
            WalkingDirection::West,
        ] {
            let len = iter_expansion(&center, &direction, FLOOR).count();
            let expected = match direction {
                WalkingDirection::North | WalkingDirection::South => TILES_X,
                _ => TILES_Y,
            };
            assert_eq!(len, expected, "{direction:?}");
        }
    }

    #[test]
    fn diagonal_expansions_add_the_column_without_the_shared_corner() {
        let center = center(100, 100);
        for direction in [
            WalkingDirection::NorthEast,
            WalkingDirection::NorthWest,
            WalkingDirection::SouthEast,
            WalkingDirection::SouthWest,
        ] {
            let positions: Vec<Position> = iter_expansion(&center, &direction, FLOOR).collect();
            assert_eq!(
                positions.len(),
                TILES_X + TILES_Y - 1,
                "{direction:?} row + column - shared corner"
            );

            let mut unique = positions.clone();
            unique.sort_by_key(|p| (p.y, p.x));
            unique.dedup();
            assert_eq!(
                unique.len(),
                positions.len(),
                "{direction:?} has no repeats"
            );
        }
    }
}
