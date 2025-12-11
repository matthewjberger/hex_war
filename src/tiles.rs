use crate::ecs::{Entity, GameWorld, HEX_POSITION, HexPosition, TILE, Tile, TileType};
use crate::hex::HexCoord;

pub fn spawn_tile(game_world: &mut GameWorld, hex_coord: HexCoord, tile_type: TileType) -> Entity {
    let entity = game_world.spawn_entities(HEX_POSITION | TILE, 1)[0];
    game_world.set_hex_position(entity, HexPosition(hex_coord));
    game_world.set_tile(
        entity,
        Tile {
            tile_type,
            faction: None,
        },
    );
    entity
}

pub fn despawn_all_tiles(game_world: &mut GameWorld) {
    let tile_entities: Vec<_> = game_world.query_entities(TILE).collect();
    game_world.despawn_entities(&tile_entities);
}
