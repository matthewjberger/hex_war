use crate::constants::{
    MAX_SOLDIERS, UNIT_DEFAULT_MOVEMENT_RANGE, UNIT_HEIGHT_OFFSET, UNIT_MOVEMENT_SPEED,
};
use crate::ecs::{
    ENGINE_ENTITY, EngineEntity, Faction, GameWorld, HEX_POSITION, HexPosition, MOVEMENT, Movement,
    UNIT, Unit, WORLD_POSITION, WorldPosition, faction_color, get_faction_morale,
};
use crate::hex::{HexCoord, hex_to_world_position};
use crate::systems::find_path;
use nightshade::prelude::*;

pub const UNIT_BASE_RADIUS: f32 = 25.0;
pub const UNIT_MAX_RADIUS: f32 = 50.0;
pub const UNIT_TEXT_HEIGHT_OFFSET: f32 = 200.0;
pub const UNIT_SELECTED_COLOR: [f32; 4] = [1.0, 0.8, 0.2, 1.0];

pub fn unit_radius_for_soldiers(soldiers: i32) -> f32 {
    let t = (soldiers as f32 / MAX_SOLDIERS as f32).clamp(0.0, 1.0);
    UNIT_BASE_RADIUS + (UNIT_MAX_RADIUS - UNIT_BASE_RADIUS) * t
}

pub fn spawn_unit(
    game_world: &mut GameWorld,
    world: &mut World,
    hex_coord: HexCoord,
    hex_width: f32,
    hex_depth: f32,
    faction: Faction,
    soldiers: i32,
) -> freecs::Entity {
    let radius = unit_radius_for_soldiers(soldiers);
    let position = hex_to_world_position(hex_coord.column, hex_coord.row, hex_width, hex_depth);
    let unit_position = nalgebra_glm::vec3(
        position.x,
        position.y + radius + UNIT_HEIGHT_OFFSET,
        position.z,
    );

    let render_entity = spawn_mesh(
        world,
        "Sphere",
        unit_position,
        nalgebra_glm::vec3(radius, radius, radius),
    );

    if let Some(material) = world.get_material_mut(render_entity) {
        material.base_color = faction_color(faction);
    }

    let font_size = font_size_for_soldiers(soldiers);
    let text_position = nalgebra_glm::vec3(
        unit_position.x,
        unit_position.y + radius + UNIT_TEXT_HEIGHT_OFFSET,
        unit_position.z,
    );
    let color = faction_color(faction);
    let text_entity = spawn_3d_billboard_text_with_properties(
        world,
        &soldiers.to_string(),
        text_position,
        TextProperties {
            font_size,
            color: nalgebra_glm::vec4(color[0], color[1], color[2], 1.0),
            alignment: TextAlignment::Center,
            outline_width: 0.15,
            outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
            smoothing: 0.15,
            ..Default::default()
        },
    );

    let morale = get_faction_morale(&game_world.resources, faction);

    let game_entity =
        game_world.spawn_entities(ENGINE_ENTITY | WORLD_POSITION | HEX_POSITION | UNIT, 1)[0];
    game_world.set_engine_entity(game_entity, EngineEntity(render_entity));
    game_world.set_world_position(game_entity, WorldPosition(unit_position));
    game_world.set_hex_position(game_entity, HexPosition(hex_coord));
    game_world.set_unit(
        game_entity,
        Unit {
            faction,
            soldiers,
            morale,
            movement_range: UNIT_DEFAULT_MOVEMENT_RANGE,
            has_moved: false,
            text_entity: Some(text_entity),
        },
    );

    game_entity
}

pub fn font_size_for_soldiers(soldiers: i32) -> f32 {
    let t = (soldiers as f32 / MAX_SOLDIERS as f32).clamp(0.0, 1.0);
    15000.0 + 5000.0 * t
}

pub fn despawn_unit(game_world: &mut GameWorld, world: &mut World, entity: freecs::Entity) {
    if let Some(unit) = game_world.get_unit(entity)
        && let Some(text_entity) = unit.text_entity
    {
        world.queue_command(WorldCommand::DespawnRecursive {
            entity: text_entity,
        });
    }
    if let Some(engine_entity) = game_world.get_engine_entity(entity) {
        world.queue_command(WorldCommand::DespawnRecursive {
            entity: engine_entity.0,
        });
    }
    game_world.despawn_entities(&[entity]);
}

pub fn move_unit_to(
    game_world: &mut GameWorld,
    unit_entity: freecs::Entity,
    destination: HexCoord,
) {
    let Some(hex_position) = game_world.get_hex_position(unit_entity) else {
        return;
    };
    let start = hex_position.0;

    let Some(path) = find_path(game_world, start, destination) else {
        return;
    };

    if path.len() < 2 {
        return;
    }

    game_world.add_components(unit_entity, MOVEMENT);
    game_world.set_movement(
        unit_entity,
        Movement {
            path,
            current_segment: 0,
            segment_progress: 0.0,
            speed: UNIT_MOVEMENT_SPEED,
        },
    );
}

pub fn unit_visual_update_system(game_world: &GameWorld, world: &mut World) {
    for entity in game_world.query_entities(UNIT | ENGINE_ENTITY | WORLD_POSITION) {
        let Some(unit) = game_world.get_unit(entity) else {
            continue;
        };
        let Some(engine_entity) = game_world.get_engine_entity(entity) else {
            continue;
        };
        let Some(world_position) = game_world.get_world_position(entity) else {
            continue;
        };

        let radius = unit_radius_for_soldiers(unit.soldiers);

        if let Some(transform) = world.get_local_transform_mut(engine_entity.0) {
            transform.scale = nalgebra_glm::vec3(radius, radius, radius);
        }
        mark_local_transform_dirty(world, engine_entity.0);

        if let Some(text_entity) = unit.text_entity {
            if let Some(text_transform) = world.get_local_transform_mut(text_entity) {
                text_transform.translation = nalgebra_glm::vec3(
                    world_position.0.x,
                    world_position.0.y + radius + UNIT_TEXT_HEIGHT_OFFSET,
                    world_position.0.z,
                );
            }
            mark_local_transform_dirty(world, text_entity);

            let font_size = font_size_for_soldiers(unit.soldiers);
            if let Some(text) = world.get_text_mut(text_entity) {
                text.properties.font_size = font_size;
                text.dirty = true;
            }
        }
    }
}
