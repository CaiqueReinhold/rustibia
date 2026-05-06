use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    agent::{UpdateElevation, WalkingDirection},
    conf::map::{TILES_X, TILES_Y},
    core::ItemConfigs,
    items::{ChangedTileQueue, Item},
    map::{minimap::MinimapData, Map, Position},
    network::{
        events::{DescribeMap, TileChanged},
        ItemStack,
    },
};

fn iter_viewport(pos: &Position, floor: u8) -> impl Iterator<Item = Position> {
    let floor_offset = pos.z as i16 - floor as i16;
    let half_w = (TILES_X / 2) as i16;
    let half_h = (TILES_Y / 2) as i16;
    let x = pos.x as i16;
    let y = pos.y as i16;

    let x_start = (x - half_w + floor_offset).max(0) as u16;
    let x_end = (x + half_w + floor_offset) as u16;
    let y_start = (y - half_h + floor_offset).max(0) as u16;
    let y_end = (y + half_h + floor_offset) as u16;
    let z = floor;

    (y_start..=y_end).flat_map(move |y| (x_start..=x_end).map(move |x| Position { x, y, z }))
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
        // For diagonals: full edge row + edge column excluding the shared corner
        WalkingDirection::NorthEast => Box::new(top_row.chain(right_col.skip(1))),
        WalkingDirection::NorthWest => Box::new(top_row.chain(left_col.skip(1))),
        WalkingDirection::SouthEast => {
            Box::new(bottom_row.chain(right_col.take(((y_end - y_start) - 1) as usize)))
        }
        WalkingDirection::SouthWest => {
            Box::new(bottom_row.chain(left_col.take(((y_end - y_start) - 1) as usize)))
        }
    }
}

fn update_tile(
    tile: &ItemStack,
    position: &Position,
    map: &mut Map,
    config: &ItemConfigs,
    minimap: &mut MinimapData,
    commands: &mut Commands,
) {
    let mut items = Vec::with_capacity(8);
    for item in tile {
        if item.is_none() {
            break;
        }
        let (item_id, amount) = item.unwrap();
        let config = config.items.get(&item_id).unwrap();
        items.push(Arc::new(Item::new(config.clone(), amount as u32)));
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
}

pub(super) fn on_describe_map(
    event: On<DescribeMap>,
    mut commands: Commands,
    config: Res<ItemConfigs>,
    mut map: ResMut<Map>,
    mut queue: ResMut<ChangedTileQueue>,
    mut minimap: ResMut<MinimapData>,
) {
    for (i, position) in iter_viewport(&event.center, event.floor).enumerate() {
        let tile = event.tiles[i];
        update_tile(
            &tile,
            &position,
            &mut map,
            &config,
            &mut minimap,
            &mut commands,
        );
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
            update_tile(&tiles[i], &position, map, config, minimap, commands);
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
    update_tile(
        &event.items,
        &event.position,
        &mut map,
        &config,
        &mut minimap,
        &mut commands,
    );
    queue.changed_positions.push_back(event.position.clone());
}
