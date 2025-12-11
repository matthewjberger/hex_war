use crate::camera::world_to_screen;
use crate::ecs::{GameWorld, HEX_POSITION, TILE};
use crate::hex::{HexCoord, hex_tiles_at_distance, hex_to_world_position, world_to_hex};
use nightshade::ecs::picking::queries::PickingRay;
use nightshade::prelude::*;
use std::collections::HashSet;

pub fn hover_system(game_world: &mut GameWorld, world: &World) {
    let mouse = &world.resources.input.mouse;
    let mouse_pos = mouse.position;

    let hex_width = game_world.resources.hex_width;
    let hex_depth = game_world.resources.hex_depth;

    let hovered_tile_coord =
        find_tile_under_cursor(game_world, world, mouse_pos, hex_width, hex_depth);
    game_world.resources.hovered_tile = hovered_tile_coord;
}

fn find_tile_under_cursor(
    game_world: &GameWorld,
    world: &World,
    mouse_pos: Vec2,
    hex_width: f32,
    hex_depth: f32,
) -> Option<HexCoord> {
    let ray = PickingRay::from_screen_position(world, mouse_pos)?;
    let rough_hit = ray.intersect_ground_plane(0.0)?;
    let rough_coord = world_to_hex(rough_hit.x, rough_hit.z, hex_width, hex_depth);

    let tile_surface_y = 5.0;

    let reference_world_pos = Vec3::new(rough_hit.x, tile_surface_y, rough_hit.z);
    let reference_screen = world_to_screen(world, reference_world_pos)?;

    let offset_world_pos = Vec3::new(rough_hit.x + hex_width, tile_surface_y, rough_hit.z);
    let offset_screen = world_to_screen(world, offset_world_pos)?;

    let tile_screen_width = ((offset_screen.x - reference_screen.x).powi(2)
        + (offset_screen.y - reference_screen.y).powi(2))
    .sqrt();

    let max_screen_distance = (tile_screen_width * 0.55).max(8.0);

    let screen_search_pixels = 200.0f32;
    let search_tiles_radius =
        ((screen_search_pixels / tile_screen_width).ceil() as i32).clamp(2, 15);

    let existing_tiles: HashSet<HexCoord> = game_world
        .query_entities(HEX_POSITION | TILE)
        .filter_map(|entity| game_world.get_hex_position(entity).map(|hex| hex.0))
        .collect();

    let mut candidates: Vec<HexCoord> = Vec::new();
    candidates.push(rough_coord);
    for distance in 1..=search_tiles_radius {
        candidates.extend(hex_tiles_at_distance(rough_coord, distance));
    }

    let mut best_tile: Option<HexCoord> = None;
    let mut best_distance_sq = f32::MAX;

    for coord in candidates {
        if !existing_tiles.contains(&coord) {
            continue;
        }

        let tile_world_pos = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);
        let tile_top_pos = Vec3::new(tile_world_pos.x, tile_surface_y, tile_world_pos.z);

        if let Some(screen_pos) = world_to_screen(world, tile_top_pos) {
            let dx = screen_pos.x - mouse_pos.x;
            let dy = screen_pos.y - mouse_pos.y;
            let distance_sq = dx * dx + dy * dy;

            if distance_sq < best_distance_sq {
                best_distance_sq = distance_sq;
                best_tile = Some(coord);
            }
        }
    }

    if best_distance_sq > max_screen_distance * max_screen_distance {
        return None;
    }

    best_tile
}
