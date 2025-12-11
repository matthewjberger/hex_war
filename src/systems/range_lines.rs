use crate::ecs::GameWorld;
use crate::hex::HexCoord;
use crate::rendering::generate_range_circle_lines;
use nightshade::prelude::*;

pub fn range_lines_system(
    game_world: &mut GameWorld,
    world: &mut World,
    range_lines_entity: Option<Entity>,
) {
    let Some(entity) = range_lines_entity else {
        return;
    };

    let current_count = game_world.resources.valid_move_tiles.len();
    let previous_count = game_world.resources.previous_valid_move_count;

    if current_count == previous_count {
        return;
    }

    if current_count == 0 {
        if let Some(visibility) = world.get_visibility_mut(entity) {
            visibility.visible = false;
        }
    } else {
        let valid_coords: Vec<HexCoord> = game_world
            .resources
            .valid_move_tiles
            .iter()
            .copied()
            .collect();

        let range_lines = generate_range_circle_lines(
            &valid_coords,
            game_world.resources.hex_width,
            game_world.resources.hex_depth,
            nalgebra_glm::vec4(1.0, 1.0, 0.0, 1.0),
        );

        if let Some(lines_component) = world.get_lines_mut(entity) {
            lines_component.lines = range_lines;
            lines_component.mark_dirty();
        }
        if let Some(visibility) = world.get_visibility_mut(entity) {
            visibility.visible = true;
        }
    }

    game_world.resources.previous_valid_move_count = current_count;
}
