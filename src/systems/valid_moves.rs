use crate::ecs::{Entity, GameWorld, HEX_POSITION, TILE, TileType, UNIT};
use crate::hex::{HexCoord, hex_neighbors, hex_to_world_position};
use std::collections::{HashMap, HashSet, VecDeque};

fn find_sea_path(game_world: &GameWorld, from: HexCoord, to: HexCoord) -> Option<Vec<HexCoord>> {
    let sea_tiles: HashSet<HexCoord> = game_world
        .query_entities(HEX_POSITION | TILE)
        .filter_map(|entity| {
            let coord = game_world.get_hex_position(entity)?.0;
            let tile = game_world.get_tile(entity)?;
            if tile.tile_type == TileType::Sea {
                Some(coord)
            } else {
                None
            }
        })
        .collect();

    let sea_neighbors_of_from: Vec<HexCoord> = hex_neighbors(from)
        .into_iter()
        .filter(|coord| sea_tiles.contains(coord))
        .collect();

    let sea_neighbors_of_to: Vec<HexCoord> = hex_neighbors(to)
        .into_iter()
        .filter(|coord| sea_tiles.contains(coord))
        .collect();

    if sea_neighbors_of_from.is_empty() || sea_neighbors_of_to.is_empty() {
        return Some(vec![from, to]);
    }

    let mut best_path: Option<Vec<HexCoord>> = None;
    let mut best_length = i32::MAX;

    for &start_sea in &sea_neighbors_of_from {
        for &end_sea in &sea_neighbors_of_to {
            let mut predecessors: HashMap<HexCoord, HexCoord> = HashMap::new();
            let mut distances: HashMap<HexCoord, i32> = HashMap::new();
            let mut queue: VecDeque<HexCoord> = VecDeque::new();

            distances.insert(start_sea, 0);
            queue.push_back(start_sea);

            while let Some(current) = queue.pop_front() {
                if current == end_sea {
                    break;
                }

                let current_dist = distances[&current];

                for neighbor in hex_neighbors(current) {
                    if !sea_tiles.contains(&neighbor) {
                        continue;
                    }
                    if distances.contains_key(&neighbor) {
                        continue;
                    }
                    distances.insert(neighbor, current_dist + 1);
                    predecessors.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }

            if let Some(&dist) = distances.get(&end_sea)
                && dist < best_length
            {
                let mut sea_path = vec![end_sea];
                let mut current = end_sea;
                while let Some(&pred) = predecessors.get(&current) {
                    sea_path.push(pred);
                    current = pred;
                }
                sea_path.reverse();

                let mut full_path = vec![from];
                full_path.extend(sea_path);
                full_path.push(to);

                best_length = dist;
                best_path = Some(full_path);
            }
        }
    }

    best_path.or_else(|| Some(vec![from, to]))
}

fn direction_alignment(
    from_coord: HexCoord,
    to_coord: HexCoord,
    goal_coord: HexCoord,
    hex_width: f32,
    hex_depth: f32,
) -> i32 {
    let from_world = hex_to_world_position(from_coord.column, from_coord.row, hex_width, hex_depth);
    let to_world = hex_to_world_position(to_coord.column, to_coord.row, hex_width, hex_depth);
    let goal_world = hex_to_world_position(goal_coord.column, goal_coord.row, hex_width, hex_depth);

    let step_x = to_world.x - from_world.x;
    let step_z = to_world.z - from_world.z;
    let goal_x = goal_world.x - from_world.x;
    let goal_z = goal_world.z - from_world.z;

    let step_len = (step_x * step_x + step_z * step_z).sqrt();
    let goal_len = (goal_x * goal_x + goal_z * goal_z).sqrt();

    if step_len < 0.001 || goal_len < 0.001 {
        return 0;
    }

    let dot = (step_x * goal_x + step_z * goal_z) / (step_len * goal_len);
    (dot * 10000.0) as i32
}

pub fn find_path(game_world: &GameWorld, from: HexCoord, to: HexCoord) -> Option<Vec<HexCoord>> {
    if from == to {
        return Some(vec![from]);
    }

    let hex_width = game_world.resources.hex_width;
    let hex_depth = game_world.resources.hex_depth;

    let passable_tiles: HashSet<HexCoord> = game_world
        .query_entities(HEX_POSITION | TILE)
        .filter_map(|entity| {
            let coord = game_world.get_hex_position(entity)?.0;
            let tile = game_world.get_tile(entity)?;
            if tile.tile_type != TileType::Sea {
                Some(coord)
            } else {
                None
            }
        })
        .collect();

    if !passable_tiles.contains(&from) || !passable_tiles.contains(&to) {
        return None;
    }

    let from_is_port = game_world
        .query_entities(HEX_POSITION | TILE)
        .any(|entity| {
            let Some(coord) = game_world.get_hex_position(entity).map(|h| h.0) else {
                return false;
            };
            let Some(tile) = game_world.get_tile(entity) else {
                return false;
            };
            coord == from && tile.tile_type == TileType::Port
        });

    let to_is_port = game_world
        .query_entities(HEX_POSITION | TILE)
        .any(|entity| {
            let Some(coord) = game_world.get_hex_position(entity).map(|h| h.0) else {
                return false;
            };
            let Some(tile) = game_world.get_tile(entity) else {
                return false;
            };
            coord == to && tile.tile_type == TileType::Port
        });

    if from_is_port && to_is_port {
        return find_sea_path(game_world, from, to);
    }

    let mut predecessors: HashMap<HexCoord, Vec<HexCoord>> = HashMap::new();
    let mut distances: HashMap<HexCoord, i32> = HashMap::new();
    let mut queue: VecDeque<HexCoord> = VecDeque::new();

    distances.insert(from, 0);
    queue.push_back(from);

    while let Some(current) = queue.pop_front() {
        let current_dist = distances[&current];

        for neighbor in hex_neighbors(current) {
            if !passable_tiles.contains(&neighbor) {
                continue;
            }

            let new_dist = current_dist + 1;

            match distances.get(&neighbor) {
                None => {
                    distances.insert(neighbor, new_dist);
                    predecessors.insert(neighbor, vec![current]);
                    queue.push_back(neighbor);
                }
                Some(&existing_dist) if existing_dist == new_dist => {
                    predecessors.get_mut(&neighbor).unwrap().push(current);
                }
                _ => {}
            }
        }
    }

    if !distances.contains_key(&to) {
        return None;
    }

    let mut path = vec![to];
    let mut current = to;

    while current != from {
        let preds = predecessors.get(&current).unwrap();

        let best_pred = if preds.len() == 1 {
            preds[0]
        } else {
            let next_in_path = if path.len() >= 2 {
                Some(path[path.len() - 2])
            } else {
                None
            };

            *preds
                .iter()
                .max_by_key(|&&pred| {
                    let alignment_to_goal =
                        direction_alignment(pred, current, to, hex_width, hex_depth);

                    let alignment_to_next = if let Some(next) = next_in_path {
                        let prev_step_x = current.column - next.column;
                        let prev_step_z = current.row - next.row;
                        let this_step_x = current.column - pred.column;
                        let this_step_z = current.row - pred.row;
                        if prev_step_x == this_step_x && prev_step_z == this_step_z {
                            10000
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    alignment_to_goal + alignment_to_next
                })
                .unwrap()
        };

        path.push(best_pred);
        current = best_pred;
    }

    path.reverse();
    Some(path)
}

pub fn calculate_valid_moves(
    game_world: &GameWorld,
    unit_entity: Entity,
    unit_hex: HexCoord,
    movement_range: i32,
) -> Vec<HexCoord> {
    let unit_positions: HashSet<HexCoord> = game_world
        .query_entities(HEX_POSITION | UNIT)
        .filter(|&entity| entity != unit_entity)
        .filter_map(|entity| game_world.get_hex_position(entity).map(|hex| hex.0))
        .collect();

    let passable_tiles: HashSet<HexCoord> = game_world
        .query_entities(HEX_POSITION | TILE)
        .filter_map(|entity| {
            let coord = game_world.get_hex_position(entity)?.0;
            let tile = game_world.get_tile(entity)?;
            if tile.tile_type != TileType::Sea {
                Some(coord)
            } else {
                None
            }
        })
        .collect();

    let port_tiles: HashSet<HexCoord> = game_world
        .query_entities(HEX_POSITION | TILE)
        .filter_map(|entity| {
            let coord = game_world.get_hex_position(entity)?.0;
            let tile = game_world.get_tile(entity)?;
            if tile.tile_type == TileType::Port {
                Some(coord)
            } else {
                None
            }
        })
        .collect();

    let starting_on_port = port_tiles.contains(&unit_hex);

    let mut distances: HashMap<HexCoord, i32> = HashMap::new();
    let mut queue: VecDeque<HexCoord> = VecDeque::new();

    distances.insert(unit_hex, 0);
    queue.push_back(unit_hex);

    while let Some(current) = queue.pop_front() {
        let current_distance = distances[&current];
        if current_distance >= movement_range {
            continue;
        }

        for neighbor in hex_neighbors(current) {
            if !passable_tiles.contains(&neighbor) {
                continue;
            }
            if distances.contains_key(&neighbor) {
                continue;
            }
            distances.insert(neighbor, current_distance + 1);
            queue.push_back(neighbor);
        }

        if starting_on_port && current == unit_hex {
            for &port_coord in &port_tiles {
                if port_coord == unit_hex {
                    continue;
                }
                if distances.contains_key(&port_coord) {
                    continue;
                }
                distances.insert(port_coord, movement_range);
            }
        }
    }

    distances
        .into_iter()
        .filter(|(coord, distance)| {
            *distance > 0 && *distance <= movement_range && !unit_positions.contains(coord)
        })
        .map(|(coord, _)| coord)
        .collect()
}

pub fn valid_moves_system(game_world: &mut GameWorld) {
    let current_selected: Option<Entity> = game_world.query_selected().next();
    let previous_selected = game_world.resources.previous_selected_unit;

    if current_selected == previous_selected {
        return;
    }

    game_world.resources.valid_move_tiles.clear();

    if let Some(unit_entity) = current_selected
        && let (Some(hex_pos), Some(unit)) = (
            game_world.get_hex_position(unit_entity),
            game_world.get_unit(unit_entity),
        )
    {
        let valid_moves =
            calculate_valid_moves(game_world, unit_entity, hex_pos.0, unit.movement_range);
        for coord in valid_moves {
            game_world.resources.valid_move_tiles.insert(coord);
        }
    }

    game_world.resources.previous_selected_unit = current_selected;
}
