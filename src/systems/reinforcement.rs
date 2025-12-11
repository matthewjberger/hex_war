use crate::constants::{CITY_REINFORCEMENT, MAX_SOLDIERS};
use crate::ecs::{
    Entity, Faction, GameEvents, GameWorld, HEX_POSITION, ReinforcementEvent, TILE, TileType, UNIT,
    faction_index,
};
use crate::hex::{HexCoord, hex_distance};
use crate::map::CAPITAL_POSITIONS;
use std::collections::HashMap;

pub struct PendingSpawn {
    pub coord: HexCoord,
    pub faction: Faction,
    pub soldiers: i32,
}

fn tile_type_name(tile_type: TileType) -> &'static str {
    match tile_type {
        TileType::Capital => "capital",
        TileType::City => "city",
        TileType::Port => "port",
        _ => "tile",
    }
}

fn get_capital_coord(faction: Faction) -> HexCoord {
    let index = faction_index(faction);
    let (col, row, _) = CAPITAL_POSITIONS[index];
    HexCoord { column: col, row }
}

pub fn reinforcement_system(
    game_world: &mut GameWorld,
    events: &mut GameEvents,
) -> Vec<PendingSpawn> {
    let current_faction = game_world.resources.current_faction;
    let mut pending_spawns = Vec::new();

    let tile_info: HashMap<HexCoord, (TileType, Option<Faction>)> = game_world
        .query_entities(HEX_POSITION | TILE)
        .filter_map(|entity| {
            let coord = game_world.get_hex_position(entity)?.0;
            let tile = game_world.get_tile(entity)?;
            Some((coord, (tile.tile_type, tile.faction)))
        })
        .collect();

    let unit_positions: HashMap<HexCoord, Entity> = game_world
        .query_entities(HEX_POSITION | UNIT)
        .filter_map(|entity| {
            let coord = game_world.get_hex_position(entity)?.0;
            Some((coord, entity))
        })
        .collect();

    for (&coord, &(tile_type, tile_faction)) in &tile_info {
        if tile_faction != Some(current_faction) {
            continue;
        }

        let reinforcement = match tile_type {
            TileType::City | TileType::Capital => CITY_REINFORCEMENT,
            _ => continue,
        };

        if let Some(&unit_entity) = unit_positions.get(&coord) {
            if let Some(unit) = game_world.get_unit(unit_entity)
                && unit.faction == current_faction
            {
                let mut unit = *unit;
                unit.soldiers = (unit.soldiers + reinforcement).min(MAX_SOLDIERS);
                game_world.set_unit(unit_entity, unit);
                events.reinforcement_events.push(ReinforcementEvent {
                    faction: current_faction,
                    soldiers: reinforcement,
                    location_name: tile_type_name(tile_type).to_string(),
                });
            }
        } else {
            pending_spawns.push(PendingSpawn {
                coord,
                faction: current_faction,
                soldiers: reinforcement,
            });
            events.reinforcement_events.push(ReinforcementEvent {
                faction: current_faction,
                soldiers: reinforcement,
                location_name: tile_type_name(tile_type).to_string(),
            });
        }
    }

    for (&coord, &(tile_type, tile_faction)) in &tile_info {
        if tile_type != TileType::Port {
            continue;
        }

        if tile_faction != Some(current_faction) {
            continue;
        }

        let port_reinforcement = 1 + (game_world.resources.rng_seed as i32 % 3);
        game_world.resources.rng_seed = game_world
            .resources
            .rng_seed
            .wrapping_mul(1103515245)
            .wrapping_add(12345);

        let mut closest_unit: Option<(Entity, i32)> = None;
        for (&unit_coord, &unit_entity) in &unit_positions {
            if let Some(unit) = game_world.get_unit(unit_entity)
                && unit.faction != current_faction
            {
                continue;
            }

            let distance = hex_distance(coord, unit_coord);
            if distance <= 3 && (closest_unit.is_none() || distance < closest_unit.unwrap().1) {
                closest_unit = Some((unit_entity, distance));
            }
        }

        if let Some((unit_entity, _)) = closest_unit
            && let Some(unit) = game_world.get_unit(unit_entity)
        {
            let mut unit = *unit;
            unit.soldiers = (unit.soldiers + port_reinforcement).min(MAX_SOLDIERS);
            game_world.set_unit(unit_entity, unit);
            events.reinforcement_events.push(ReinforcementEvent {
                faction: current_faction,
                soldiers: port_reinforcement,
                location_name: "port".to_string(),
            });
        }
    }

    let territory_count = tile_info
        .values()
        .filter(|(tile_type, faction)| {
            *faction == Some(current_faction) && *tile_type != TileType::Sea
        })
        .count();
    let territory_bonus = (territory_count / 10) as i32;

    if territory_bonus > 0 {
        let capital_coord = get_capital_coord(current_faction);

        if let Some(&unit_entity) = unit_positions.get(&capital_coord) {
            if let Some(unit) = game_world.get_unit(unit_entity)
                && unit.faction == current_faction
            {
                let mut unit = *unit;
                unit.soldiers = (unit.soldiers + territory_bonus).min(MAX_SOLDIERS);
                game_world.set_unit(unit_entity, unit);
                events.reinforcement_events.push(ReinforcementEvent {
                    faction: current_faction,
                    soldiers: territory_bonus,
                    location_name: "territory".to_string(),
                });
            }
        } else if tile_info
            .get(&capital_coord)
            .map(|(_, f)| *f == Some(current_faction))
            .unwrap_or(false)
        {
            pending_spawns.push(PendingSpawn {
                coord: capital_coord,
                faction: current_faction,
                soldiers: territory_bonus.max(1),
            });
            events.reinforcement_events.push(ReinforcementEvent {
                faction: current_faction,
                soldiers: territory_bonus.max(1),
                location_name: "territory".to_string(),
            });
        }
    }

    pending_spawns
}
