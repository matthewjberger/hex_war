use nightshade::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HexCoord {
    pub column: i32,
    pub row: i32,
}

pub fn hex_to_cube(coord: HexCoord) -> (i32, i32, i32) {
    let x = coord.column;
    let z = coord.row - (coord.column - (coord.column & 1)) / 2;
    let y = -x - z;
    (x, y, z)
}

pub fn hex_from_cube(x: i32, _y: i32, z: i32) -> HexCoord {
    let column = x;
    let row = z + (x - (x & 1)) / 2;
    HexCoord { column, row }
}

pub fn hex_distance(from: HexCoord, to: HexCoord) -> i32 {
    let (x1, y1, z1) = hex_to_cube(from);
    let (x2, y2, z2) = hex_to_cube(to);
    ((x1 - x2).abs() + (y1 - y2).abs() + (z1 - z2).abs()) / 2
}

pub fn hex_neighbors(coord: HexCoord) -> [HexCoord; 6] {
    let column = coord.column;
    let row = coord.row;
    let is_odd_column = column.abs() % 2 != 0;

    if is_odd_column {
        [
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
        [
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

pub fn hex_tiles_in_range(center: HexCoord, range: i32) -> Vec<HexCoord> {
    let mut result = Vec::new();
    for distance in 1..=range {
        result.extend(hex_tiles_at_distance(center, distance));
    }
    result
}

pub fn hex_tiles_at_distance(center: HexCoord, distance: i32) -> Vec<HexCoord> {
    if distance == 0 {
        return vec![center];
    }
    let mut result = Vec::new();
    let (cx, cy, cz) = hex_to_cube(center);
    for x in -distance..=distance {
        for y in (-distance).max(-x - distance)..=(distance).min(-x + distance) {
            let z = -x - y;
            if x.abs() + y.abs() + z.abs() == distance * 2 {
                result.push(hex_from_cube(cx + x, cy + y, cz + z));
            }
        }
    }
    result
}

pub fn hex_to_world_position(column: i32, row: i32, hex_width: f32, hex_height: f32) -> Vec3 {
    let is_flat_top = hex_width > hex_height;

    if is_flat_top {
        let horizontal_spacing = hex_width * 0.75;
        let vertical_spacing = hex_height;

        let offset = if column.abs() % 2 != 0 {
            vertical_spacing * 0.5
        } else {
            0.0
        };

        nalgebra_glm::vec3(
            column as f32 * horizontal_spacing,
            0.0,
            row as f32 * vertical_spacing + offset,
        )
    } else {
        let horizontal_spacing = hex_width;
        let vertical_spacing = hex_height * 0.75;

        let offset = if row.abs() % 2 != 0 {
            horizontal_spacing * 0.5
        } else {
            0.0
        };

        nalgebra_glm::vec3(
            column as f32 * horizontal_spacing + offset,
            0.0,
            row as f32 * vertical_spacing,
        )
    }
}

pub fn world_to_hex(world_x: f32, world_z: f32, hex_width: f32, hex_height: f32) -> HexCoord {
    let is_flat_top = hex_width > hex_height;

    if is_flat_top {
        let horizontal_spacing = hex_width * 0.75;
        let vertical_spacing = hex_height;

        let approx_column = (world_x / horizontal_spacing).round() as i32;
        let offset = if approx_column.abs() % 2 != 0 {
            vertical_spacing * 0.5
        } else {
            0.0
        };
        let approx_row = ((world_z - offset) / vertical_spacing).round() as i32;

        let mut best_coord = HexCoord {
            column: approx_column,
            row: approx_row,
        };
        let mut best_dist = f32::MAX;

        for dc in -1..=1 {
            for dr in -1..=1 {
                let candidate = HexCoord {
                    column: approx_column + dc,
                    row: approx_row + dr,
                };
                let candidate_pos =
                    hex_to_world_position(candidate.column, candidate.row, hex_width, hex_height);
                let dist_sq =
                    (candidate_pos.x - world_x).powi(2) + (candidate_pos.z - world_z).powi(2);
                if dist_sq < best_dist {
                    best_dist = dist_sq;
                    best_coord = candidate;
                }
            }
        }
        best_coord
    } else {
        let horizontal_spacing = hex_width;
        let vertical_spacing = hex_height * 0.75;

        let approx_row = (world_z / vertical_spacing).round() as i32;
        let offset = if approx_row.abs() % 2 != 0 {
            horizontal_spacing * 0.5
        } else {
            0.0
        };
        let approx_column = ((world_x - offset) / horizontal_spacing).round() as i32;

        let mut best_coord = HexCoord {
            column: approx_column,
            row: approx_row,
        };
        let mut best_dist = f32::MAX;

        for dc in -1..=1 {
            for dr in -1..=1 {
                let candidate = HexCoord {
                    column: approx_column + dc,
                    row: approx_row + dr,
                };
                let candidate_pos =
                    hex_to_world_position(candidate.column, candidate.row, hex_width, hex_height);
                let dist_sq =
                    (candidate_pos.x - world_x).powi(2) + (candidate_pos.z - world_z).powi(2);
                if dist_sq < best_dist {
                    best_dist = dist_sq;
                    best_coord = candidate;
                }
            }
        }
        best_coord
    }
}
