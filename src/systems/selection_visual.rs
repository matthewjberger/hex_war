use crate::ecs::{GameWorld, faction_color};
use crate::systems::UNIT_SELECTED_COLOR;
use nightshade::prelude::*;

pub fn selection_visual_system(game_world: &GameWorld, world: &mut World) {
    let current_selected: Option<freecs::Entity> = game_world.query_selected().next();
    let previous_selected = game_world.resources.previous_selected_unit;

    if current_selected == previous_selected {
        return;
    }

    if let Some(prev_entity) = previous_selected
        && let Some(unit) = game_world.get_unit(prev_entity)
        && let Some(engine_entity) = game_world.get_engine_entity(prev_entity)
        && let Some(material) = world.get_material_mut(engine_entity.0)
    {
        material.base_color = faction_color(unit.faction);
    }

    if let Some(curr_entity) = current_selected
        && let Some(engine_entity) = game_world.get_engine_entity(curr_entity)
        && let Some(material) = world.get_material_mut(engine_entity.0)
    {
        material.base_color = UNIT_SELECTED_COLOR;
    }
}
