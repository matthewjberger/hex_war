use crate::hex::{HexCoord, hex_to_world_position};
use nightshade::ecs::world::components::Line;
use nightshade::prelude::*;

pub fn generate_hex_outline(
    center: Vec3,
    hex_width: f32,
    hex_height: f32,
    y_offset: f32,
) -> Vec<Line> {
    generate_hex_outline_with_color(
        center,
        hex_width,
        hex_height,
        y_offset,
        nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
    )
}

pub fn generate_hex_outline_with_color(
    center: Vec3,
    hex_width: f32,
    hex_height: f32,
    y_offset: f32,
    color: Vec4,
) -> Vec<Line> {
    let mut lines = Vec::new();
    let is_flat_top = hex_width > hex_height;

    let vertices: Vec<Vec3> = if is_flat_top {
        let half_width = hex_width / 2.0;
        let quarter_width = hex_width / 4.0;
        let half_height = hex_height / 2.0;
        vec![
            nalgebra_glm::vec3(center.x + half_width, y_offset, center.z),
            nalgebra_glm::vec3(center.x + quarter_width, y_offset, center.z + half_height),
            nalgebra_glm::vec3(center.x - quarter_width, y_offset, center.z + half_height),
            nalgebra_glm::vec3(center.x - half_width, y_offset, center.z),
            nalgebra_glm::vec3(center.x - quarter_width, y_offset, center.z - half_height),
            nalgebra_glm::vec3(center.x + quarter_width, y_offset, center.z - half_height),
        ]
    } else {
        let half_width = hex_width / 2.0;
        let half_height = hex_height / 2.0;
        let quarter_height = hex_height / 4.0;
        vec![
            nalgebra_glm::vec3(center.x, y_offset, center.z - half_height),
            nalgebra_glm::vec3(center.x + half_width, y_offset, center.z - quarter_height),
            nalgebra_glm::vec3(center.x + half_width, y_offset, center.z + quarter_height),
            nalgebra_glm::vec3(center.x, y_offset, center.z + half_height),
            nalgebra_glm::vec3(center.x - half_width, y_offset, center.z + quarter_height),
            nalgebra_glm::vec3(center.x - half_width, y_offset, center.z - quarter_height),
        ]
    };

    for vertex_index in 0..6 {
        let start = vertices[vertex_index];
        let end = vertices[(vertex_index + 1) % 6];
        lines.push(Line { start, end, color });
    }

    lines
}

pub fn generate_range_circle_lines(
    tiles_in_range: &[HexCoord],
    hex_width: f32,
    hex_depth: f32,
    color: Vec4,
) -> Vec<Line> {
    let mut lines = Vec::new();
    let y_offset = 10.0;

    for coord in tiles_in_range {
        let tile_center = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);
        let hex_lines = generate_hex_outline(tile_center, hex_width, hex_depth, y_offset);
        for mut line in hex_lines {
            line.color = color;
            lines.push(line);
        }
    }

    lines
}
