use crate::ecs::GameWorld;
use crate::hex::{HexCoord, hex_to_world_position};
use crate::instancing::InstancedTileGroup;
use crate::rendering::generate_hex_outline;
use nightshade::ecs::world::components::Line;
use nightshade::prelude::*;
use std::collections::HashSet;

const DEFAULT_TINT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const HOVER_TINT: [f32; 4] = [1.3, 1.3, 1.0, 1.0];
const VALID_MOVE_TINT: [f32; 4] = [0.8, 1.2, 0.8, 1.0];
const HOVER_VALID_TINT: [f32; 4] = [1.0, 1.5, 0.7, 1.0];

pub fn tile_highlight_system(
    game_world: &mut GameWorld,
    world: &mut World,
    instanced_tile_groups: &[InstancedTileGroup],
) {
    let hovered_tile = game_world.resources.hovered_tile;
    let valid_move_tiles = &game_world.resources.valid_move_tiles;

    let mut currently_highlighted: HashSet<HexCoord> = valid_move_tiles.clone();
    if let Some(coord) = hovered_tile {
        currently_highlighted.insert(coord);
    }

    let tiles_to_reset: Vec<HexCoord> = game_world
        .resources
        .previously_highlighted
        .difference(&currently_highlighted)
        .copied()
        .collect();

    let tiles_to_highlight: Vec<HexCoord> = currently_highlighted
        .difference(&game_world.resources.previously_highlighted)
        .copied()
        .collect();

    let tiles_needing_tint_update: Vec<HexCoord> = currently_highlighted
        .intersection(&game_world.resources.previously_highlighted)
        .copied()
        .collect();

    let hover_changed = hovered_tile != game_world.resources.previous_hovered_tile;

    if tiles_to_reset.is_empty()
        && tiles_to_highlight.is_empty()
        && (!hover_changed || tiles_needing_tint_update.is_empty())
    {
        return;
    }

    for group in instanced_tile_groups {
        let Some(instanced_mesh) = world.get_instanced_mesh_mut(group.entity) else {
            continue;
        };

        for coord in &tiles_to_reset {
            if let Some(&instance_index) = group.coord_to_instance.get(coord) {
                instanced_mesh.set_instance_tint(instance_index, DEFAULT_TINT);
            }
        }

        for coord in &tiles_to_highlight {
            if let Some(&instance_index) = group.coord_to_instance.get(coord) {
                let is_hovered = hovered_tile == Some(*coord);
                let is_valid_move = valid_move_tiles.contains(coord);
                let tint = match (is_hovered, is_valid_move) {
                    (true, true) => HOVER_VALID_TINT,
                    (true, false) => HOVER_TINT,
                    (false, true) => VALID_MOVE_TINT,
                    (false, false) => DEFAULT_TINT,
                };
                instanced_mesh.set_instance_tint(instance_index, tint);
            }
        }

        if hover_changed {
            for coord in &tiles_needing_tint_update {
                if let Some(&instance_index) = group.coord_to_instance.get(coord) {
                    let is_hovered = hovered_tile == Some(*coord);
                    let is_valid_move = valid_move_tiles.contains(coord);
                    let tint = match (is_hovered, is_valid_move) {
                        (true, true) => HOVER_VALID_TINT,
                        (true, false) => HOVER_TINT,
                        (false, true) => VALID_MOVE_TINT,
                        (false, false) => DEFAULT_TINT,
                    };
                    instanced_mesh.set_instance_tint(instance_index, tint);
                }
            }
        }
    }

    game_world.resources.previously_highlighted = currently_highlighted;
    game_world.resources.previous_hovered_tile = hovered_tile;
}

pub fn hover_outline_system(
    game_world: &GameWorld,
    world: &mut World,
    hover_outline_entity: Option<Entity>,
) {
    let Some(entity) = hover_outline_entity else {
        return;
    };

    let hovered_tile = game_world.resources.hovered_tile;

    match hovered_tile {
        Some(coord) => {
            let hex_width = game_world.resources.hex_width;
            let hex_depth = game_world.resources.hex_depth;
            let tile_center = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);
            let outline_lines = generate_hex_outline(tile_center, hex_width, hex_depth, 6.0);
            let yellow_lines: Vec<Line> = outline_lines
                .into_iter()
                .map(|mut line| {
                    line.color = nalgebra_glm::vec4(1.0, 1.0, 0.0, 1.0);
                    line
                })
                .collect();

            if let Some(lines_component) = world.get_lines_mut(entity) {
                lines_component.lines = yellow_lines;
                lines_component.mark_dirty();
            }
            if let Some(visibility) = world.get_visibility_mut(entity) {
                visibility.visible = true;
            }
        }
        None => {
            if let Some(visibility) = world.get_visibility_mut(entity) {
                visibility.visible = false;
            }
        }
    }
}
