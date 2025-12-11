use crate::constants::INITIAL_SOLDIERS;
use crate::ecs::{GameWorld, TileType};
use crate::hex::{HexCoord, hex_to_world_position};
use crate::instancing::{InstancedTileGroup, create_instanced_tiles};
use crate::map::{CAPITAL_POSITIONS, GeneratedMap, TileFeature, generate_map};
use crate::rendering::generate_hex_outline;
use crate::systems::spawn_unit;
use crate::tiles::spawn_tile;
use nightshade::ecs::prefab::Prefab;
use nightshade::ecs::world::components::Line;
use nightshade::ecs::world::{
    GLOBAL_TRANSFORM, LINES, LOCAL_TRANSFORM, LOCAL_TRANSFORM_DIRTY, VISIBILITY,
};
use nightshade::prelude::*;
use std::collections::HashMap;

const HEX_OUTLINE_HEIGHT: f32 = 5.0;
const SEA_EXTENSION: i32 = 30;

pub struct MapEntities {
    pub instanced_tile_groups: Vec<InstancedTileGroup>,
    pub lines_entity: Entity,
    pub boundary_lines_entity: Entity,
    pub range_lines_entity: Entity,
    pub hover_outline_entity: Entity,
    pub port_label_entities: Vec<Entity>,
}

pub fn generate_game_map(
    game_world: &mut GameWorld,
    world: &mut World,
    tile_prefabs: &HashMap<TileType, Prefab>,
) -> MapEntities {
    use crate::constants::{MAP_HEIGHT, MAP_WIDTH};

    game_world.resources.rng_seed = rand::rng().random();
    let generated = generate_map(game_world.resources.rng_seed);

    let hex_width = game_world.resources.hex_width;
    let hex_depth = game_world.resources.hex_depth;

    let mut all_hex_lines: Vec<Line> = Vec::new();
    let mut tile_positions: Vec<(HexCoord, TileType)> = Vec::new();
    let mut port_coords: Vec<HexCoord> = Vec::new();

    for (&coord, &base_type) in &generated.tiles {
        let tile_type = determine_tile_type(base_type, coord, &generated);
        tile_positions.push((coord, tile_type));

        if tile_type == TileType::Port {
            port_coords.push(coord);
        }

        spawn_tile(game_world, coord, tile_type);

        let position = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);
        let hex_lines = generate_hex_outline(position, hex_width, hex_depth, HEX_OUTLINE_HEIGHT);
        all_hex_lines.extend(hex_lines);
    }

    for column in -SEA_EXTENSION..(MAP_WIDTH + SEA_EXTENSION) {
        for row in -SEA_EXTENSION..(MAP_HEIGHT + SEA_EXTENSION) {
            let coord = HexCoord { column, row };
            if generated.tiles.contains_key(&coord) {
                continue;
            }
            tile_positions.push((coord, TileType::Sea));

            let position = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);
            let hex_lines =
                generate_hex_outline(position, hex_width, hex_depth, HEX_OUTLINE_HEIGHT);
            all_hex_lines.extend(hex_lines);
        }
    }

    let instanced_tile_groups =
        create_instanced_tiles(world, tile_prefabs, &tile_positions, hex_width, hex_depth);

    let lines_entity = spawn_lines_entity(world, all_hex_lines);
    let boundary_lines =
        generate_playable_boundary_lines(MAP_WIDTH, MAP_HEIGHT, hex_width, hex_depth);
    let boundary_lines_entity = spawn_lines_entity(world, boundary_lines);
    let range_lines_entity = spawn_hidden_lines_entity(world);
    let hover_outline_entity = spawn_hidden_lines_entity(world);

    let port_label_entities = spawn_port_labels(world, &port_coords, hex_width, hex_depth);

    spawn_initial_units(game_world, world, hex_width, hex_depth);

    MapEntities {
        instanced_tile_groups,
        lines_entity,
        boundary_lines_entity,
        range_lines_entity,
        hover_outline_entity,
        port_label_entities,
    }
}

fn determine_tile_type(base_type: TileType, coord: HexCoord, generated: &GeneratedMap) -> TileType {
    if base_type == TileType::Sea {
        return TileType::Sea;
    }

    match generated.features.get(&coord) {
        Some(TileFeature::Capital(_)) => TileType::Capital,
        Some(TileFeature::City) => TileType::City,
        Some(TileFeature::Port) => TileType::Port,
        None => base_type,
    }
}

fn spawn_lines_entity(world: &mut World, lines: Vec<Line>) -> Entity {
    let entity = world.spawn_entities(
        LINES | VISIBILITY | LOCAL_TRANSFORM | GLOBAL_TRANSFORM | LOCAL_TRANSFORM_DIRTY,
        1,
    )[0];
    if let Some(lines_component) = world.get_lines_mut(entity) {
        lines_component.lines = lines;
        lines_component.mark_dirty();
    }
    entity
}

fn spawn_hidden_lines_entity(world: &mut World) -> Entity {
    let entity = world.spawn_entities(
        LINES | VISIBILITY | LOCAL_TRANSFORM | GLOBAL_TRANSFORM | LOCAL_TRANSFORM_DIRTY,
        1,
    )[0];
    if let Some(lines_component) = world.get_lines_mut(entity) {
        lines_component.lines = Vec::new();
        lines_component.mark_dirty();
    }
    if let Some(visibility) = world.get_visibility_mut(entity) {
        visibility.visible = false;
    }
    entity
}

const PORT_LABEL_HEIGHT: f32 = 100.0;

fn spawn_port_labels(
    world: &mut World,
    port_coords: &[HexCoord],
    hex_width: f32,
    hex_depth: f32,
) -> Vec<Entity> {
    let mut entities = Vec::new();

    for coord in port_coords {
        let position = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);
        let label_position =
            nalgebra_glm::vec3(position.x, position.y + PORT_LABEL_HEIGHT, position.z);

        let entity = spawn_3d_billboard_text_with_properties(
            world,
            "PORT",
            label_position,
            TextProperties {
                font_size: 8000.0,
                color: nalgebra_glm::vec4(0.3, 0.7, 1.0, 1.0),
                alignment: TextAlignment::Center,
                outline_width: 0.15,
                outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
                smoothing: 0.15,
                ..Default::default()
            },
        );

        entities.push(entity);
    }

    entities
}

fn spawn_initial_units(
    game_world: &mut GameWorld,
    world: &mut World,
    hex_width: f32,
    hex_depth: f32,
) {
    for (col, row, faction) in CAPITAL_POSITIONS {
        let coord = HexCoord { column: col, row };
        spawn_unit(
            game_world,
            world,
            coord,
            hex_width,
            hex_depth,
            faction,
            INITIAL_SOLDIERS,
        );
    }
}

fn generate_playable_boundary_lines(
    map_width: i32,
    map_height: i32,
    hex_width: f32,
    hex_depth: f32,
) -> Vec<Line> {
    let mut lines = Vec::new();
    let boundary_color = nalgebra_glm::vec4(1.0, 0.5, 0.0, 1.0);
    let boundary_y = 10.0;

    let is_flat_top = hex_width > hex_depth;

    if is_flat_top {
        let half_width = hex_width / 2.0;
        let quarter_width = hex_width / 4.0;
        let half_depth = hex_depth / 2.0;

        let horizontal_spacing = hex_width * 0.75;
        let vertical_spacing = hex_depth;

        let mut boundary_points: Vec<Vec3> = Vec::new();

        for column in 0..map_width {
            let row = 0;
            let offset = if column.abs() % 2 != 0 {
                vertical_spacing * 0.5
            } else {
                0.0
            };
            let center_x = column as f32 * horizontal_spacing;
            let center_z = row as f32 * vertical_spacing + offset;

            if column == 0 {
                boundary_points.push(nalgebra_glm::vec3(
                    center_x - half_width,
                    boundary_y,
                    center_z,
                ));
                boundary_points.push(nalgebra_glm::vec3(
                    center_x - quarter_width,
                    boundary_y,
                    center_z - half_depth,
                ));
            }
            boundary_points.push(nalgebra_glm::vec3(
                center_x + quarter_width,
                boundary_y,
                center_z - half_depth,
            ));
            boundary_points.push(nalgebra_glm::vec3(
                center_x + half_width,
                boundary_y,
                center_z,
            ));
        }

        for row in 0..map_height {
            let column = map_width - 1;
            let offset = if column.abs() % 2 != 0 {
                vertical_spacing * 0.5
            } else {
                0.0
            };
            let center_x = column as f32 * horizontal_spacing;
            let center_z = row as f32 * vertical_spacing + offset;

            if row == 0 {
                boundary_points.push(nalgebra_glm::vec3(
                    center_x + quarter_width,
                    boundary_y,
                    center_z + half_depth,
                ));
            } else {
                boundary_points.push(nalgebra_glm::vec3(
                    center_x + half_width,
                    boundary_y,
                    center_z,
                ));
                boundary_points.push(nalgebra_glm::vec3(
                    center_x + quarter_width,
                    boundary_y,
                    center_z + half_depth,
                ));
            }
        }

        for column in (0..map_width).rev() {
            let row = map_height - 1;
            let offset = if column.abs() % 2 != 0 {
                vertical_spacing * 0.5
            } else {
                0.0
            };
            let center_x = column as f32 * horizontal_spacing;
            let center_z = row as f32 * vertical_spacing + offset;

            if column == map_width - 1 {
                boundary_points.push(nalgebra_glm::vec3(
                    center_x + quarter_width,
                    boundary_y,
                    center_z + half_depth,
                ));
                boundary_points.push(nalgebra_glm::vec3(
                    center_x - quarter_width,
                    boundary_y,
                    center_z + half_depth,
                ));
            } else {
                boundary_points.push(nalgebra_glm::vec3(
                    center_x + half_width,
                    boundary_y,
                    center_z,
                ));
                boundary_points.push(nalgebra_glm::vec3(
                    center_x + quarter_width,
                    boundary_y,
                    center_z + half_depth,
                ));
                boundary_points.push(nalgebra_glm::vec3(
                    center_x - quarter_width,
                    boundary_y,
                    center_z + half_depth,
                ));
            }
            boundary_points.push(nalgebra_glm::vec3(
                center_x - half_width,
                boundary_y,
                center_z,
            ));
        }

        for row in (0..map_height).rev() {
            let column: i32 = 0;
            let offset = if column.abs() % 2 != 0 {
                vertical_spacing * 0.5
            } else {
                0.0
            };
            let center_x = column as f32 * horizontal_spacing;
            let center_z = row as f32 * vertical_spacing + offset;

            if row == map_height - 1 {
                boundary_points.push(nalgebra_glm::vec3(
                    center_x - quarter_width,
                    boundary_y,
                    center_z - half_depth,
                ));
            } else {
                boundary_points.push(nalgebra_glm::vec3(
                    center_x - half_width,
                    boundary_y,
                    center_z,
                ));
                boundary_points.push(nalgebra_glm::vec3(
                    center_x - quarter_width,
                    boundary_y,
                    center_z - half_depth,
                ));
            }
        }

        for index in 0..boundary_points.len() {
            let start = boundary_points[index];
            let end = boundary_points[(index + 1) % boundary_points.len()];
            lines.push(Line {
                start,
                end,
                color: boundary_color,
            });
        }
    }

    lines
}

pub fn despawn_map_entities(world: &mut World, entities: &mut MapEntities) {
    for group in entities.instanced_tile_groups.drain(..) {
        world.queue_command(WorldCommand::DespawnRecursive {
            entity: group.entity,
        });
    }
    world.queue_command(WorldCommand::DespawnRecursive {
        entity: entities.lines_entity,
    });
    world.queue_command(WorldCommand::DespawnRecursive {
        entity: entities.boundary_lines_entity,
    });
    world.queue_command(WorldCommand::DespawnRecursive {
        entity: entities.range_lines_entity,
    });
    world.queue_command(WorldCommand::DespawnRecursive {
        entity: entities.hover_outline_entity,
    });
    for entity in entities.port_label_entities.drain(..) {
        world.queue_command(WorldCommand::DespawnRecursive { entity });
    }
}
