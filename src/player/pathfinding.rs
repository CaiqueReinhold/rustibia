use pathfinding::prelude::astar;

use bevy::prelude::*;

use crate::{
    agent::WalkingDirection,
    map::{Position, minimap::MinimapData},
};

#[derive(Resource)]
pub struct AutoWalkTarget(pub Position);

const MAX_PATH_STEPS: usize = 50;
const UNVISITED_COST: u32 = 10;

fn is_passable(minimap: &MinimapData, pos: &Position) -> bool {
    match minimap.get_tile(pos) {
        None => true,
        Some(tile) => tile.friction != Some(0),
    }
}

fn tile_cost(minimap: &MinimapData, pos: &Position) -> u32 {
    match minimap.get_tile(pos) {
        None => UNVISITED_COST,
        Some(tile) => match tile.friction {
            None | Some(0) => UNVISITED_COST,
            Some(n) => n as u32,
        },
    }
}

fn successors(pos: &Position, minimap: &MinimapData) -> Vec<(Position, u32)> {
    use WalkingDirection::*;
    let dirs = [
        North, South, East, West, NorthEast, NorthWest, SouthEast, SouthWest,
    ];
    let mut out = Vec::with_capacity(8);
    for dir in dirs {
        let next = pos.clone() + dir;
        if !is_passable(minimap, &next) {
            continue;
        }
        let base = tile_cost(minimap, &next);
        let cost = match dir {
            North | South | East | West => base,
            _ => base * 3,
        };
        out.push((next, cost));
    }
    out
}

fn heuristic(pos: &Position, goal: &Position) -> u32 {
    pos.x.abs_diff(goal.x).max(pos.y.abs_diff(goal.y)) as u32
}

/// Returns true if `a` and `b` are exactly 1 tile apart (8-directional) on the same Z floor.
pub fn is_adjacent(a: &Position, b: &Position) -> bool {
    a.z == b.z && a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1 && a != b
}

pub fn compute_path(
    from: &Position,
    to: &Position,
    minimap: &MinimapData,
) -> Option<Vec<WalkingDirection>> {
    // Short-circuit if target is unreachably far
    if heuristic(from, to) > MAX_PATH_STEPS as u32 {
        return None;
    }

    let (path, _cost) = astar(
        from,
        |pos| successors(pos, minimap),
        |pos| heuristic(pos, to),
        |pos| pos == to,
    )?;

    if path.len().saturating_sub(1) > MAX_PATH_STEPS {
        return None;
    }

    Some(path.windows(2).map(|w| pos_to_dir(&w[0], &w[1])).collect())
}

/// Computes a path to any tile adjacent to `target`.
/// Returns `Some(vec![])` if already adjacent or standing on `target`, `None` if unreachable within the step cap.
/// Assumes `from` and `target` are on the same Z floor; cross-floor inputs will always return `None`.
pub fn compute_path_to_adjacent(
    from: &Position,
    target: &Position,
    minimap: &MinimapData,
) -> Option<Vec<WalkingDirection>> {
    if from == target {
        return Some(vec![]);
    }
    if is_adjacent(from, target) {
        return Some(vec![]);
    }

    // Short-circuit: if even the target itself is too far, no adjacent tile can be closer
    if heuristic(from, target).saturating_sub(1) > MAX_PATH_STEPS as u32 {
        return None;
    }

    let (path, _cost) = astar(
        from,
        |pos| successors(pos, minimap),
        |pos| heuristic(pos, target).saturating_sub(1),
        |pos| is_adjacent(pos, target),
    )?;

    if path.len().saturating_sub(1) > MAX_PATH_STEPS {
        return None;
    }

    Some(path.windows(2).map(|w| pos_to_dir(&w[0], &w[1])).collect())
}

/// Computes a path to any tile reachable from both `a` and `b` (i.e., adjacent to or standing on each).
/// Returns `Some(vec![])` if the player already qualifies, `None` if no such tile exists or it's unreachable.
/// Both `a` and `b` must be on the same Z floor as `from`.
pub fn compute_path_adjacent_to_both(
    from: &Position,
    a: &Position,
    b: &Position,
    minimap: &MinimapData,
) -> Option<Vec<WalkingDirection>> {
    if a.z != b.z || a.z != from.z {
        return None;
    }
    // No tile can be within 1 of both if they are more than 2 apart (Chebyshev).
    if heuristic(a, b) > 2 {
        return None;
    }

    let reaches = |p: &Position, x: &Position| -> bool { p == x || is_adjacent(p, x) };

    if reaches(from, a) && reaches(from, b) {
        return Some(vec![]);
    }

    // Admissible heuristic: at least one of the two endpoints must still be reached.
    let h = |p: &Position| -> u32 {
        heuristic(p, a)
            .saturating_sub(1)
            .max(heuristic(p, b).saturating_sub(1))
    };

    if h(from) > MAX_PATH_STEPS as u32 {
        return None;
    }

    let (path, _cost) = astar(
        from,
        |pos| successors(pos, minimap),
        h,
        |pos| reaches(pos, a) && reaches(pos, b),
    )?;

    if path.len().saturating_sub(1) > MAX_PATH_STEPS {
        return None;
    }

    Some(path.windows(2).map(|w| pos_to_dir(&w[0], &w[1])).collect())
}

fn pos_to_dir(from: &Position, to: &Position) -> WalkingDirection {
    match (to.x as i32 - from.x as i32, to.y as i32 - from.y as i32) {
        (0, -1) => WalkingDirection::North,
        (0, 1) => WalkingDirection::South,
        (1, 0) => WalkingDirection::East,
        (-1, 0) => WalkingDirection::West,
        (1, -1) => WalkingDirection::NorthEast,
        (-1, -1) => WalkingDirection::NorthWest,
        (1, 1) => WalkingDirection::SouthEast,
        (-1, 1) => WalkingDirection::SouthWest,
        _ => WalkingDirection::North,
    }
}
