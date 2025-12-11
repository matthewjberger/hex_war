use crate::ecs::{Entity, GameWorld, HEX_POSITION, UNIT};
use crate::hex::HexCoord;

pub fn get_selected_unit(game_world: &GameWorld) -> Option<Entity> {
    game_world.query_selected().next()
}

pub fn get_unit_at_tile(game_world: &GameWorld, coord: HexCoord) -> Option<Entity> {
    game_world
        .query_entities(HEX_POSITION | UNIT)
        .find(|&entity| {
            game_world
                .get_hex_position(entity)
                .map(|hex| hex.0 == coord)
                .unwrap_or(false)
        })
}

pub fn select_unit(game_world: &mut GameWorld, unit_entity: Entity) {
    clear_selection(game_world);
    game_world.add_selected(unit_entity);
}

pub fn clear_selection(game_world: &mut GameWorld) {
    let selected_entities: Vec<_> = game_world.query_selected().collect();
    for selected in selected_entities {
        game_world.remove_selected(selected);
    }
    game_world.resources.valid_move_tiles.clear();
}
