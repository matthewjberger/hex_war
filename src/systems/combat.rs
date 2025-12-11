use crate::ecs::{
    Faction, GameWorld, HEX_POSITION, TILE, modify_faction_morale, tile_defense_bonus,
};
use crate::hex::HexCoord;
use crate::systems::{despawn_unit, move_unit_to};
use nightshade::prelude::*;

pub struct CombatResult {
    pub attacker_faction: Faction,
    pub defender_faction: Faction,
    pub attacker_survived: bool,
    pub defender_survived: bool,
}

pub fn resolve_combat(
    game_world: &mut GameWorld,
    world: &mut World,
    attacker_entity: freecs::Entity,
    defender_entity: freecs::Entity,
) -> Option<CombatResult> {
    let attacker = game_world.get_unit(attacker_entity).copied()?;
    let defender = game_world.get_unit(defender_entity).copied()?;
    let defender_hex = game_world.get_hex_position(defender_entity)?.0;

    let attacker_faction = attacker.faction;
    let defender_faction = defender.faction;

    let defense_bonus = get_defense_bonus_at(game_world, defender_hex);

    let attacker_strength = attacker.soldiers as f32 * (1.0 + attacker.morale as f32 / 100.0);
    let defender_strength =
        defender.soldiers as f32 * (1.0 + defender.morale as f32 / 100.0) * defense_bonus;

    let attacker_wins = attacker_strength > defender_strength;

    if attacker_wins {
        let attacker_casualties = (defender.soldiers as f32 * 0.7).floor() as i32;
        let attacker_new_soldiers = attacker.soldiers - attacker_casualties;

        despawn_unit(game_world, world, defender_entity);

        let attacker_survived = attacker_new_soldiers > 0;
        if attacker_survived {
            if let Some(unit) = game_world.get_unit_mut(attacker_entity) {
                unit.soldiers = attacker_new_soldiers;
                unit.has_moved = true;
            }
            move_unit_to(game_world, attacker_entity, defender_hex);
            update_tile_ownership(game_world, defender_hex, attacker_faction);
        } else {
            despawn_unit(game_world, world, attacker_entity);
        }

        modify_faction_morale(&mut game_world.resources, attacker_faction, 2);
        modify_faction_morale(&mut game_world.resources, defender_faction, -2);

        Some(CombatResult {
            attacker_faction,
            defender_faction,
            attacker_survived,
            defender_survived: false,
        })
    } else {
        let defender_casualties = (attacker.soldiers as f32 * 0.5).floor() as i32;
        let defender_new_soldiers = defender.soldiers - defender_casualties;

        despawn_unit(game_world, world, attacker_entity);

        let defender_survived = defender_new_soldiers > 0;
        if defender_survived {
            if let Some(unit) = game_world.get_unit_mut(defender_entity) {
                unit.soldiers = defender_new_soldiers;
            }
        } else {
            despawn_unit(game_world, world, defender_entity);
        }

        modify_faction_morale(&mut game_world.resources, defender_faction, 2);
        modify_faction_morale(&mut game_world.resources, attacker_faction, -2);

        Some(CombatResult {
            attacker_faction,
            defender_faction,
            attacker_survived: false,
            defender_survived,
        })
    }
}

fn get_defense_bonus_at(game_world: &GameWorld, coord: HexCoord) -> f32 {
    game_world
        .query_entities(HEX_POSITION | TILE)
        .find_map(|entity| {
            let hex = game_world.get_hex_position(entity)?;
            if hex.0 == coord {
                let tile = game_world.get_tile(entity)?;
                Some(tile_defense_bonus(tile.tile_type))
            } else {
                None
            }
        })
        .unwrap_or(1.0)
}

fn update_tile_ownership(game_world: &mut GameWorld, coord: HexCoord, faction: Faction) {
    let tile_entity = game_world
        .query_entities(HEX_POSITION | TILE)
        .find(|&entity| {
            game_world
                .get_hex_position(entity)
                .map(|hex| hex.0 == coord)
                .unwrap_or(false)
        });

    if let Some(entity) = tile_entity
        && let Some(tile) = game_world.get_tile_mut(entity)
    {
        tile.faction = Some(faction);
    }
}
