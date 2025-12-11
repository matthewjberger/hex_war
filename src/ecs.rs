use crate::hex::HexCoord;
use crate::map::MapGenParams;
use nightshade::prelude::*;
use std::collections::HashSet;

pub use freecs::Entity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Difficulty {
    #[default]
    Easy,
    Normal,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Faction {
    #[default]
    Redosia,
    Violetnam,
    Bluegaria,
    Greenland,
}

pub fn next_faction(faction: Faction) -> Faction {
    match faction {
        Faction::Redosia => Faction::Violetnam,
        Faction::Violetnam => Faction::Bluegaria,
        Faction::Bluegaria => Faction::Greenland,
        Faction::Greenland => Faction::Redosia,
    }
}

pub fn faction_color(faction: Faction) -> [f32; 4] {
    match faction {
        Faction::Redosia => [0.8, 0.2, 0.2, 1.0],
        Faction::Violetnam => [0.6, 0.2, 0.8, 1.0],
        Faction::Bluegaria => [0.2, 0.4, 0.8, 1.0],
        Faction::Greenland => [0.2, 0.8, 0.2, 1.0],
    }
}

pub fn faction_index(faction: Faction) -> usize {
    match faction {
        Faction::Redosia => 0,
        Faction::Violetnam => 1,
        Faction::Bluegaria => 2,
        Faction::Greenland => 3,
    }
}

pub fn faction_name(faction: Faction) -> &'static str {
    match faction {
        Faction::Redosia => "Redosia",
        Faction::Violetnam => "Violetnam",
        Faction::Bluegaria => "Bluegaria",
        Faction::Greenland => "Greenland",
    }
}

freecs::ecs! {
    GameWorld {
        engine_entity: EngineEntity => ENGINE_ENTITY,
        world_position: WorldPosition => WORLD_POSITION,
        hex_position: HexPosition => HEX_POSITION,
        unit: Unit => UNIT,
        movement: Movement => MOVEMENT,
        tile: Tile => TILE,
        floating_popup: FloatingPopup => FLOATING_POPUP,
    }
    Tags {
        selected => SELECTED,
    }
    GameResources {
        hex_width: f32,
        hex_depth: f32,
        rng_seed: u32,
        map_params: MapGenParams,
        needs_regeneration: bool,
        valid_move_tiles: HashSet<HexCoord>,
        hovered_tile: Option<HexCoord>,
        previously_highlighted: HashSet<HexCoord>,
        previous_hovered_tile: Option<HexCoord>,
        previous_selected_unit: Option<freecs::Entity>,
        previous_valid_move_count: usize,
        current_faction: Faction,
        actions_remaining: u8,
        turn_number: u32,
        faction_eliminated: [bool; 4],
        faction_morale: [i32; 4],
        capital_owners: [Option<Faction>; 4],
        speech_used: bool,
        turn_order: Vec<freecs::Entity>,
        current_unit_index: usize,
        game_speed: f32,
        difficulty: Difficulty,
    }
}

pub fn get_faction_morale(resources: &GameResources, faction: Faction) -> i32 {
    resources.faction_morale[faction_index(faction)]
}

pub fn modify_faction_morale(resources: &mut GameResources, faction: Faction, delta: i32) {
    let index = faction_index(faction);
    resources.faction_morale[index] = (resources.faction_morale[index] + delta).clamp(-50, 50);
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EngineEntity(pub Entity);

#[derive(Debug, Clone, Copy, Default)]
pub struct WorldPosition(pub Vec3);

#[derive(Debug, Clone, Copy, Default)]
pub struct HexPosition(pub HexCoord);

#[derive(Debug, Clone, Copy, Default)]
pub struct Unit {
    pub faction: Faction,
    pub soldiers: i32,
    pub morale: i32,
    pub movement_range: i32,
    pub has_moved: bool,
    pub text_entity: Option<Entity>,
}

#[derive(Debug, Clone, Default)]
pub struct Movement {
    pub path: Vec<HexCoord>,
    pub current_segment: usize,
    pub segment_progress: f32,
    pub speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TileType {
    Sea,
    #[default]
    Land,
    Forest,
    City,
    Port,
    Capital,
}

pub fn tile_defense_bonus(tile_type: TileType) -> f32 {
    match tile_type {
        TileType::Capital => 1.2,
        TileType::City => 1.1,
        TileType::Forest => 1.15,
        TileType::Port => 1.05,
        _ => 1.0,
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tile {
    pub tile_type: TileType,
    pub faction: Option<Faction>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FloatingPopup {
    pub text_entity: Entity,
    pub lifetime: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CombatEvent {
    pub attacker_faction: Faction,
    pub defender_faction: Faction,
    pub attacker_survived: bool,
    pub defender_survived: bool,
}

#[derive(Debug, Clone)]
pub struct ReinforcementEvent {
    pub faction: Faction,
    pub soldiers: i32,
    pub location_name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct SpeechEvent {
    pub faction: Faction,
}

#[derive(Debug, Clone, Copy)]
pub struct FactionEliminatedEvent {
    pub faction: Faction,
}

#[derive(Default)]
pub struct GameEvents {
    pub combat_events: Vec<CombatEvent>,
    pub reinforcement_events: Vec<ReinforcementEvent>,
    pub speech_events: Vec<SpeechEvent>,
    pub faction_eliminated_events: Vec<FactionEliminatedEvent>,
}
