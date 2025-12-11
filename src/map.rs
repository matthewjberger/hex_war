use crate::constants::{MAP_HEIGHT, MAP_WIDTH};
use crate::ecs::{Faction, TileType};
use crate::hex::{HexCoord, hex_distance, hex_from_cube, hex_tiles_in_range, hex_to_cube};
use std::collections::{HashMap, HashSet, VecDeque};

pub const CAPITAL_POSITIONS: [(i32, i32, Faction); 4] = [
    (2, 2, Faction::Redosia),
    (28, 2, Faction::Violetnam),
    (28, 18, Faction::Bluegaria),
    (2, 18, Faction::Greenland),
];

#[derive(Clone)]
pub struct MapGenParams {
    pub map_width: i32,
    pub map_height: i32,
}

impl Default for MapGenParams {
    fn default() -> Self {
        Self {
            map_width: MAP_WIDTH,
            map_height: MAP_HEIGHT,
        }
    }
}

pub struct GeneratedMap {
    pub tiles: HashMap<HexCoord, TileType>,
    pub features: HashMap<HexCoord, TileFeature>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TileFeature {
    Capital(Faction),
    City,
    Port,
}

struct Rng {
    state: u32,
}

fn rng_next(rng: &mut Rng) -> u32 {
    rng.state = rng.state.wrapping_mul(1103515245).wrapping_add(12345);
    rng.state
}

fn rng_next_range(rng: &mut Rng, max: u32) -> u32 {
    rng_next(rng) % max
}

fn rng_shuffle<T>(rng: &mut Rng, slice: &mut [T]) {
    for index in (1..slice.len()).rev() {
        let swap_index = rng_next_range(rng, (index + 1) as u32) as usize;
        slice.swap(index, swap_index);
    }
}

fn get_hex_neighbors(coord: HexCoord) -> Vec<HexCoord> {
    let column = coord.column;
    let row = coord.row;
    let is_odd_column = column.abs() % 2 != 0;

    if is_odd_column {
        vec![
            HexCoord {
                column,
                row: row - 1,
            },
            HexCoord {
                column: column + 1,
                row,
            },
            HexCoord {
                column: column + 1,
                row: row + 1,
            },
            HexCoord {
                column,
                row: row + 1,
            },
            HexCoord {
                column: column - 1,
                row: row + 1,
            },
            HexCoord {
                column: column - 1,
                row,
            },
        ]
    } else {
        vec![
            HexCoord {
                column,
                row: row - 1,
            },
            HexCoord {
                column: column + 1,
                row: row - 1,
            },
            HexCoord {
                column: column + 1,
                row,
            },
            HexCoord {
                column,
                row: row + 1,
            },
            HexCoord {
                column: column - 1,
                row,
            },
            HexCoord {
                column: column - 1,
                row: row - 1,
            },
        ]
    }
}

fn is_in_bounds(coord: HexCoord, width: i32, height: i32) -> bool {
    coord.column >= 0 && coord.column < width && coord.row >= 0 && coord.row < height
}

fn count_land_neighbors(
    coord: HexCoord,
    tiles: &HashMap<HexCoord, TileType>,
    width: i32,
    height: i32,
) -> i32 {
    let mut count = 0;
    for neighbor in get_hex_neighbors(coord) {
        if is_in_bounds(neighbor, width, height) && tiles.get(&neighbor) == Some(&TileType::Land) {
            count += 1;
        }
    }
    count
}

fn flood_fill_land(
    start: HexCoord,
    tiles: &HashMap<HexCoord, TileType>,
    width: i32,
    height: i32,
) -> HashSet<HexCoord> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    if tiles.get(&start) != Some(&TileType::Land) {
        return visited;
    }

    queue.push_back(start);
    visited.insert(start);

    while let Some(current) = queue.pop_front() {
        for neighbor in get_hex_neighbors(current) {
            if is_in_bounds(neighbor, width, height)
                && !visited.contains(&neighbor)
                && tiles.get(&neighbor) == Some(&TileType::Land)
            {
                visited.insert(neighbor);
                queue.push_back(neighbor);
            }
        }
    }

    visited
}

fn find_path_between_landmasses(
    from_land: &HashSet<HexCoord>,
    to_land: &HashSet<HexCoord>,
    tiles: &HashMap<HexCoord, TileType>,
    width: i32,
    height: i32,
) -> Vec<HexCoord> {
    let mut best_path = Vec::new();
    let mut best_distance = i32::MAX;

    for &from_coord in from_land {
        for &to_coord in to_land {
            let distance = hex_distance(from_coord, to_coord);
            if distance < best_distance {
                best_distance = distance;
                best_path = get_line_between(from_coord, to_coord);
            }
        }
    }

    best_path
        .into_iter()
        .filter(|coord| {
            is_in_bounds(*coord, width, height) && tiles.get(coord) == Some(&TileType::Sea)
        })
        .collect()
}

fn get_line_between(from: HexCoord, to: HexCoord) -> Vec<HexCoord> {
    let mut result = Vec::new();
    let distance = hex_distance(from, to);

    if distance == 0 {
        return vec![from];
    }

    let (from_x, from_y, from_z) = hex_to_cube(from);
    let (to_x, to_y, to_z) = hex_to_cube(to);

    for step in 0..=distance {
        let t = step as f32 / distance as f32;
        let x = lerp(from_x as f32, to_x as f32, t).round() as i32;
        let y = lerp(from_y as f32, to_y as f32, t).round() as i32;
        let z = lerp(from_z as f32, to_z as f32, t).round() as i32;
        result.push(hex_from_cube(x, y, z));
    }

    result
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn is_adjacent_to_sea(
    coord: HexCoord,
    tiles: &HashMap<HexCoord, TileType>,
    width: i32,
    height: i32,
) -> bool {
    for neighbor in get_hex_neighbors(coord) {
        if !is_in_bounds(neighbor, width, height) || tiles.get(&neighbor) == Some(&TileType::Sea) {
            return true;
        }
    }
    false
}

fn is_passable_land(tile_type: TileType) -> bool {
    matches!(tile_type, TileType::Land | TileType::Forest)
}

pub fn generate_map(seed: u32) -> GeneratedMap {
    let width = MAP_WIDTH;
    let height = MAP_HEIGHT;
    let mut rng = Rng { state: seed };
    let mut tiles: HashMap<HexCoord, TileType> = HashMap::new();

    let capital_coords: Vec<HexCoord> = CAPITAL_POSITIONS
        .iter()
        .map(|(col, row, _)| HexCoord {
            column: *col,
            row: *row,
        })
        .collect();

    for row in 0..height {
        for column in 0..width {
            tiles.insert(HexCoord { column, row }, TileType::Sea);
        }
    }

    for &capital in &capital_coords {
        tiles.insert(capital, TileType::Land);
        for neighbor in hex_tiles_in_range(capital, 2) {
            if is_in_bounds(neighbor, width, height) {
                tiles.insert(neighbor, TileType::Land);
            }
        }
    }

    for _ in 0..8 {
        let mut sea_hexes: Vec<HexCoord> = tiles
            .iter()
            .filter(|(coord, tile_type)| {
                **tile_type == TileType::Sea && is_in_bounds(**coord, width, height)
            })
            .map(|(coord, _)| *coord)
            .collect();

        rng_shuffle(&mut rng, &mut sea_hexes);

        for coord in sea_hexes {
            let land_neighbors = count_land_neighbors(coord, &tiles, width, height);

            if land_neighbors >= 1 {
                let conversion_chance = (land_neighbors as u32) * 25;
                if rng_next_range(&mut rng, 100) < conversion_chance {
                    tiles.insert(coord, TileType::Land);
                }
            }
        }
    }

    let land_hexes: Vec<HexCoord> = tiles
        .iter()
        .filter(|(_, tile_type)| **tile_type == TileType::Land)
        .map(|(coord, _)| *coord)
        .collect();

    for coord in land_hexes {
        let min_capital_dist = capital_coords
            .iter()
            .map(|cap| hex_distance(coord, *cap))
            .min()
            .unwrap_or(0);

        if min_capital_dist > 5 {
            let land_neighbors = count_land_neighbors(coord, &tiles, width, height);

            let should_carve = land_neighbors <= 1 && rng_next_range(&mut rng, 100) < 15;

            if should_carve {
                tiles.insert(coord, TileType::Sea);
            }
        }
    }

    for row in 0..height {
        for column in 0..width {
            let coord = HexCoord { column, row };
            if column == 0 || column == width - 1 || row == 0 || row == height - 1 {
                tiles.insert(coord, TileType::Sea);
            }
        }
    }

    for &capital in &capital_coords {
        tiles.insert(capital, TileType::Land);
        for neighbor in hex_tiles_in_range(capital, 2) {
            if is_in_bounds(neighbor, width, height) {
                tiles.insert(neighbor, TileType::Land);
            }
        }
    }

    let first_capital = capital_coords[0];
    let mut connected_land = flood_fill_land(first_capital, &tiles, width, height);

    for &capital in &capital_coords[1..] {
        if !connected_land.contains(&capital) {
            let capital_land = flood_fill_land(capital, &tiles, width, height);
            let bridge =
                find_path_between_landmasses(&connected_land, &capital_land, &tiles, width, height);
            for coord in &bridge {
                tiles.insert(*coord, TileType::Land);
                connected_land.insert(*coord);
                for neighbor in get_hex_neighbors(*coord) {
                    if is_in_bounds(neighbor, width, height)
                        && tiles.get(&neighbor) == Some(&TileType::Sea)
                    {
                        tiles.insert(neighbor, TileType::Land);
                        connected_land.insert(neighbor);
                    }
                }
            }
            for coord in capital_land {
                connected_land.insert(coord);
            }
        }
    }

    let all_coords: Vec<HexCoord> = tiles.keys().copied().collect();
    for coord in all_coords {
        if tiles.get(&coord) == Some(&TileType::Land) {
            let land_neighbors = count_land_neighbors(coord, &tiles, width, height);
            if land_neighbors == 0 {
                tiles.insert(coord, TileType::Sea);
            }
        } else if tiles.get(&coord) == Some(&TileType::Sea) {
            let sea_neighbors = 6 - count_land_neighbors(coord, &tiles, width, height);
            if sea_neighbors == 0 {
                tiles.insert(coord, TileType::Land);
            }
        }
    }

    let land_tiles: Vec<HexCoord> = tiles
        .iter()
        .filter(|(coord, tile_type)| {
            **tile_type == TileType::Land
                && capital_coords
                    .iter()
                    .all(|cap| hex_distance(**coord, *cap) > 2)
        })
        .map(|(coord, _)| *coord)
        .collect();

    for coord in &land_tiles {
        let roll = rng_next_range(&mut rng, 100);
        if roll < 20 {
            tiles.insert(*coord, TileType::Forest);
        }
    }

    let mut features: HashMap<HexCoord, TileFeature> = HashMap::new();

    for (col, row, faction) in CAPITAL_POSITIONS {
        features.insert(HexCoord { column: col, row }, TileFeature::Capital(faction));
    }

    let target_cities = 8 + rng_next_range(&mut rng, 5) as i32;
    let mut city_coords: Vec<HexCoord> = Vec::new();

    let mut candidate_hexes: Vec<HexCoord> = tiles
        .iter()
        .filter(|(coord, tile_type)| {
            is_passable_land(**tile_type)
                && !features.contains_key(coord)
                && capital_coords
                    .iter()
                    .all(|cap| hex_distance(**coord, *cap) > 1)
                && !is_adjacent_to_sea(**coord, &tiles, width, height)
        })
        .map(|(coord, _)| *coord)
        .collect();

    rng_shuffle(&mut rng, &mut candidate_hexes);

    for coord in candidate_hexes {
        if city_coords.len() >= target_cities as usize {
            break;
        }

        let far_enough_from_cities = city_coords
            .iter()
            .all(|city| hex_distance(coord, *city) > 2);
        if far_enough_from_cities {
            features.insert(coord, TileFeature::City);
            city_coords.push(coord);
        }
    }

    let target_ports = 4 + rng_next_range(&mut rng, 3) as i32;
    let mut port_coords: Vec<HexCoord> = Vec::new();

    let mut coastal_hexes: Vec<HexCoord> = tiles
        .iter()
        .filter(|(coord, tile_type)| {
            is_passable_land(**tile_type)
                && !features.contains_key(coord)
                && is_adjacent_to_sea(**coord, &tiles, width, height)
        })
        .map(|(coord, _)| *coord)
        .collect();

    rng_shuffle(&mut rng, &mut coastal_hexes);

    for coord in coastal_hexes {
        if port_coords.len() >= target_ports as usize {
            break;
        }

        let far_enough_from_ports = port_coords
            .iter()
            .all(|port| hex_distance(coord, *port) > 2);
        if far_enough_from_ports {
            features.insert(coord, TileFeature::Port);
            port_coords.push(coord);
        }
    }

    GeneratedMap { tiles, features }
}
