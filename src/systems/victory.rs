use crate::ecs::{
    Faction, FactionEliminatedEvent, GameEvents, GameWorld, HEX_POSITION, TILE, TileType, UNIT,
    faction_index,
};
use crate::hex::HexCoord;
use crate::map::CAPITAL_POSITIONS;
use nightshade::prelude::*;

pub enum GameResult {
    Ongoing,
    Victory(Faction),
}

fn faction_from_index(index: usize) -> Faction {
    match index {
        0 => Faction::Redosia,
        1 => Faction::Violetnam,
        2 => Faction::Bluegaria,
        _ => Faction::Greenland,
    }
}

fn get_capital_coord(faction: Faction) -> HexCoord {
    let index = faction_index(faction);
    let (col, row, _) = CAPITAL_POSITIONS[index];
    HexCoord { column: col, row }
}

pub fn victory_system(
    game_world: &mut GameWorld,
    world: &mut World,
    events: &mut GameEvents,
) -> GameResult {
    for faction_idx in 0..4 {
        if game_world.resources.faction_eliminated[faction_idx] {
            continue;
        }

        let faction = faction_from_index(faction_idx);
        let capital_coord = get_capital_coord(faction);

        let capital_owner = game_world
            .query_entities(HEX_POSITION | TILE)
            .find_map(|entity| {
                let hex = game_world.get_hex_position(entity)?;
                if hex.0 == capital_coord {
                    let tile = game_world.get_tile(entity)?;
                    if tile.tile_type == TileType::Capital {
                        return tile.faction;
                    }
                }
                None
            });

        if let Some(owner) = capital_owner
            && owner != faction
        {
            game_world.resources.faction_eliminated[faction_idx] = true;
            events
                .faction_eliminated_events
                .push(FactionEliminatedEvent { faction });

            let units_to_remove: Vec<_> = game_world
                .query_entities(UNIT)
                .filter(|&entity| {
                    game_world
                        .get_unit(entity)
                        .map(|u| u.faction == faction)
                        .unwrap_or(false)
                })
                .collect();

            for entity in units_to_remove {
                crate::systems::despawn_unit(game_world, world, entity);
            }

            for entity in game_world
                .query_entities(HEX_POSITION | TILE)
                .collect::<Vec<_>>()
            {
                if let Some(tile) = game_world.get_tile(entity)
                    && tile.faction == Some(faction)
                {
                    let mut tile = *tile;
                    tile.faction = None;
                    game_world.set_tile(entity, tile);
                }
            }
        }
    }

    let alive_count = game_world
        .resources
        .faction_eliminated
        .iter()
        .filter(|&&eliminated| !eliminated)
        .count();

    if alive_count == 1 {
        for (index, &eliminated) in game_world.resources.faction_eliminated.iter().enumerate() {
            if !eliminated {
                return GameResult::Victory(faction_from_index(index));
            }
        }
    }

    GameResult::Ongoing
}
