use crate::constants::UNIT_HEIGHT_OFFSET;
use crate::ecs::{ENGINE_ENTITY, GameWorld, HEX_POSITION, MOVEMENT, UNIT, WORLD_POSITION};
use crate::hex::{HexCoord, hex_to_world_position};
use crate::systems::{UNIT_TEXT_HEIGHT_OFFSET, unit_radius_for_soldiers};
use nightshade::prelude::*;

pub fn movement_system(game_world: &mut GameWorld, world: &mut World, delta_time: f32) {
    let hex_width = game_world.resources.hex_width;
    let hex_depth = game_world.resources.hex_depth;
    let game_speed = game_world.resources.game_speed;

    let mut completed_entities: Vec<(freecs::Entity, HexCoord)> = Vec::new();
    let mut segment_completed: Vec<(freecs::Entity, HexCoord, f32)> = Vec::new();
    let mut transform_updates = Vec::new();

    game_world
        .query_mut()
        .with(MOVEMENT | WORLD_POSITION | ENGINE_ENTITY | UNIT | HEX_POSITION)
        .iter(|entity, table, index| {
            let movement = &mut table.movement[index];
            let current_hex = table.hex_position[index].0;
            let soldiers = table.unit[index].soldiers;
            let radius = unit_radius_for_soldiers(soldiers);

            if movement.path.is_empty() || movement.current_segment >= movement.path.len() - 1 {
                let final_hex = movement.path.last().copied().unwrap_or(current_hex);
                completed_entities.push((entity, final_hex));
                return;
            }

            let from_hex = movement.path[movement.current_segment];
            let to_hex = movement.path[movement.current_segment + 1];

            let from_world =
                hex_to_world_position(from_hex.column, from_hex.row, hex_width, hex_depth);
            let to_world = hex_to_world_position(to_hex.column, to_hex.row, hex_width, hex_depth);

            let from_position = nalgebra_glm::vec3(
                from_world.x,
                from_world.y + radius + UNIT_HEIGHT_OFFSET,
                from_world.z,
            );
            let to_position = nalgebra_glm::vec3(
                to_world.x,
                to_world.y + radius + UNIT_HEIGHT_OFFSET,
                to_world.z,
            );

            let new_progress = movement.segment_progress + delta_time * movement.speed * game_speed;
            let t = new_progress.clamp(0.0, 1.0);
            let smooth_t = t * t * (3.0 - 2.0 * t);

            let current_position = nalgebra_glm::lerp(&from_position, &to_position, smooth_t);

            movement.segment_progress = new_progress;
            table.world_position[index].0 = current_position;

            let text_entity = table.unit[index].text_entity;
            transform_updates.push((
                entity,
                table.engine_entity[index],
                current_position,
                text_entity,
                radius,
            ));

            if new_progress >= 1.0 {
                segment_completed.push((entity, to_hex, new_progress - 1.0));
            }
        });

    for (_, engine_entity, position, text_entity, radius) in transform_updates {
        if let Some(transform) = world.get_local_transform_mut(engine_entity.0) {
            transform.translation = position;
        }
        mark_local_transform_dirty(world, engine_entity.0);

        if let Some(text_ent) = text_entity {
            if let Some(text_transform) = world.get_local_transform_mut(text_ent) {
                text_transform.translation = nalgebra_glm::vec3(
                    position.x,
                    position.y + radius + UNIT_TEXT_HEIGHT_OFFSET,
                    position.z,
                );
            }
            mark_local_transform_dirty(world, text_ent);
        }
    }

    for (entity, reached_hex, excess_progress) in segment_completed {
        let (final_hex, is_complete) = {
            if let Some(movement) = game_world.get_movement_mut(entity) {
                movement.current_segment += 1;
                let mut remaining_progress = excess_progress;
                let mut current_hex = reached_hex;

                while remaining_progress >= 1.0
                    && movement.current_segment < movement.path.len() - 1
                {
                    current_hex = movement.path[movement.current_segment + 1];
                    movement.current_segment += 1;
                    remaining_progress -= 1.0;
                }

                movement.segment_progress = remaining_progress;

                let is_complete = movement.current_segment >= movement.path.len() - 1;
                let final_hex = if is_complete {
                    movement.path.last().copied().unwrap_or(current_hex)
                } else {
                    current_hex
                };
                (final_hex, is_complete)
            } else {
                continue;
            }
        };

        if let Some(hex_pos) = game_world.get_hex_position_mut(entity) {
            hex_pos.0 = final_hex;
        }

        if is_complete {
            completed_entities.push((entity, final_hex));
        }
    }

    for (entity, final_hex) in completed_entities {
        if let Some(hex_pos) = game_world.get_hex_position_mut(entity) {
            hex_pos.0 = final_hex;
        }
        game_world.remove_components(entity, MOVEMENT);
    }
}
