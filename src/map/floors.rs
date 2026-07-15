use bevy::prelude::*;

use crate::{
    agent::WalkingDirection,
    conf::map::{BASE_FLOOR, MAX_FLOOR, MIN_FLOOR},
    map::{Map, Position},
    player::components::Player,
};

#[derive(Resource, Debug)]
pub struct FloorEntities {
    pub floors: [Entity; (MAX_FLOOR + 1) as usize],
}

pub fn setup_floors(mut commands: Commands) {
    let mut floors = Vec::new();
    for _ in MIN_FLOOR..=MAX_FLOOR {
        floors.push(commands.spawn(Transform::default()).id());
    }
    commands.insert_resource(FloorEntities {
        floors: floors.try_into().unwrap(),
    });
}

fn has_oclusion(dir: WalkingDirection, pos: &Position, floor: u8, map: &Map) -> bool {
    let floor_offset = ((BASE_FLOOR as i32) - (floor as i32)) as u16;
    let offset_pos = Position::new(
        match dir {
            WalkingDirection::East => pos.x + floor_offset,
            WalkingDirection::West => pos.x - floor_offset,
            _ => pos.x,
        },
        match dir {
            WalkingDirection::North => pos.y - floor_offset,
            WalkingDirection::South => pos.y + floor_offset,
            _ => pos.y,
        },
        floor,
    );

    if map.is_bottom(&offset_pos) && matches!(dir, WalkingDirection::East | WalkingDirection::South)
    {
        return true;
    }

    (!map.block_sight(&(pos.clone() + dir)) || (floor as i32 - pos.z as i32) < -1)
        && map.is_ground(&offset_pos)
}

fn is_floor_visible(map: &Map, pos: &Position, floor: u8) -> bool {
    if (pos.z <= BASE_FLOOR) && (floor > BASE_FLOOR) {
        return false;
    }

    if (pos.z > BASE_FLOOR) && (floor <= BASE_FLOOR) {
        return false;
    }

    if pos.z == floor {
        return true;
    }

    if has_oclusion(WalkingDirection::North, pos, floor, map) {
        return false;
    }

    if has_oclusion(WalkingDirection::East, pos, floor, map) {
        return false;
    }

    if has_oclusion(WalkingDirection::South, pos, floor, map) {
        return false;
    }

    if has_oclusion(WalkingDirection::West, pos, floor, map) {
        return false;
    }

    true
}

pub fn update_floors_visibility(
    mut commands: Commands,
    position_q: Query<&Position, (With<Player>, Changed<Position>)>,
    floor_ents: Res<FloorEntities>,
    map: Res<Map>,
) {
    let Ok(position) = position_q.single() else {
        return;
    };

    if position.z <= BASE_FLOOR {
        let mut z = position.z as i8;
        while z >= MIN_FLOOR as i8 {
            let floor_entity = floor_ents.floors[z as usize];
            if is_floor_visible(&map, position, z as u8) {
                commands.entity(floor_entity).insert(Visibility::Visible);
            } else {
                break;
            }
            z -= 1;
        }
        if z > 0 {
            for z in MIN_FLOOR..=z as u8 {
                let floor_entity = floor_ents.floors[z as usize];
                commands.entity(floor_entity).insert(Visibility::Hidden);
            }
        }
        for z in (position.z + 1)..=BASE_FLOOR {
            let floor_entity = floor_ents.floors[z as usize];
            commands.entity(floor_entity).insert(Visibility::Visible);
        }
        for z in BASE_FLOOR + 1..=MAX_FLOOR {
            let floor_entity = floor_ents.floors[z as usize];
            commands.entity(floor_entity).insert(Visibility::Hidden);
        }
    } else {
        let mut z = position.z;
        while z > BASE_FLOOR {
            let floor_entity = floor_ents.floors[z as usize];
            if is_floor_visible(&map, position, z) {
                commands.entity(floor_entity).insert(Visibility::Visible);
            } else {
                break;
            }
            z -= 1;
        }
        for z in z..BASE_FLOOR {
            let floor_entity = floor_ents.floors[z as usize];
            commands.entity(floor_entity).insert(Visibility::Hidden);
        }
        for z in position.z..MAX_FLOOR {
            let floor_entity = floor_ents.floors[z as usize];
            commands.entity(floor_entity).insert(Visibility::Visible);
        }
        for z in MIN_FLOOR..=BASE_FLOOR {
            let floor_entity = floor_ents.floors[z as usize];
            commands.entity(floor_entity).insert(Visibility::Hidden);
        }
    }
}
