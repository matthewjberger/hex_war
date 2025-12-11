use crate::ecs::{Faction, GameWorld, HEX_POSITION, TILE, TileType, UNIT, modify_faction_morale};
use crate::hex::HexCoord;
use std::collections::HashMap;

pub struct TileCapture {
    pub coord: HexCoord,
    pub tile_type: TileType,
    pub faction: Faction,
}

fn morale_change_for_capture(tile_type: TileType, was_enemy: bool) -> i32 {
    match (tile_type, was_enemy) {
        (TileType::Capital, true) => 10,
        (TileType::City, true) => 3,
        (TileType::City, false) => 2,
        (TileType::Port, true) => 2,
        (TileType::Port, false) => 1,
        (_, true) => 1,
        (_, false) => 0,
    }
}

fn morale_change_for_loss(tile_type: TileType) -> i32 {
    match tile_type {
        TileType::City | TileType::Capital => -3,
        TileType::Port => -2,
        _ => -1,
    }
}

pub fn tile_ownership_system(game_world: &mut GameWorld) -> Vec<TileCapture> {
    let unit_positions: HashMap<HexCoord, Faction> = game_world
        .query_entities(HEX_POSITION | UNIT)
        .filter_map(|entity| {
            let coord = game_world.get_hex_position(entity)?.0;
            let unit = game_world.get_unit(entity)?;
            Some((coord, unit.faction))
        })
        .collect();

    let mut morale_changes: Vec<(Faction, i32)> = Vec::new();
    let mut captures: Vec<TileCapture> = Vec::new();

    for entity in game_world
        .query_entities(HEX_POSITION | TILE)
        .collect::<Vec<_>>()
    {
        let Some(coord) = game_world.get_hex_position(entity).map(|h| h.0) else {
            continue;
        };

        if let Some(&unit_faction) = unit_positions.get(&coord)
            && let Some(tile) = game_world.get_tile(entity)
        {
            if tile.tile_type == TileType::Sea {
                continue;
            }

            let old_owner = tile.faction;
            if old_owner == Some(unit_faction) {
                continue;
            }

            let was_enemy = old_owner.is_some();
            let gain = morale_change_for_capture(tile.tile_type, was_enemy);
            if gain > 0 {
                morale_changes.push((unit_faction, gain));
            }

            if let Some(old_faction) = old_owner {
                let loss = morale_change_for_loss(tile.tile_type);
                morale_changes.push((old_faction, loss));
            }

            if matches!(
                tile.tile_type,
                TileType::City | TileType::Port | TileType::Capital
            ) {
                captures.push(TileCapture {
                    coord,
                    tile_type: tile.tile_type,
                    faction: unit_faction,
                });
            }

            let mut tile = *tile;
            tile.faction = Some(unit_faction);
            game_world.set_tile(entity, tile);
        }
    }

    for (faction, delta) in morale_changes {
        modify_faction_morale(&mut game_world.resources, faction, delta);
    }

    captures
}
