use crate::ecs::{Faction, GameWorld, faction_color, faction_name};
use nightshade::prelude::*;

#[derive(Default)]
pub struct GameHud {
    pub turn_text: Option<Entity>,
    pub faction_text: Option<Entity>,
    pub actions_text: Option<Entity>,
    pub instructions_text: Option<Entity>,
    pub speed_text: Option<Entity>,
}

pub fn spawn_game_hud(world: &mut World) -> GameHud {
    let turn_props = TextProperties {
        font_size: 28.0,
        color: nalgebra_glm::vec4(1.0, 1.0, 1.0, 1.0),
        alignment: TextAlignment::Left,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    let faction_props = TextProperties {
        font_size: 32.0,
        color: nalgebra_glm::vec4(1.0, 1.0, 1.0, 1.0),
        alignment: TextAlignment::Left,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    let actions_props = TextProperties {
        font_size: 24.0,
        color: nalgebra_glm::vec4(0.9, 0.9, 0.7, 1.0),
        alignment: TextAlignment::Left,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    let turn_text = spawn_hud_text_with_properties(
        world,
        "Turn 1",
        HudAnchor::TopLeft,
        nalgebra_glm::vec2(15.0, 15.0),
        turn_props,
    );

    let faction_text = spawn_hud_text_with_properties(
        world,
        "Redosia",
        HudAnchor::TopLeft,
        nalgebra_glm::vec2(15.0, 50.0),
        faction_props,
    );

    let actions_text = spawn_hud_text_with_properties(
        world,
        "Actions: 5",
        HudAnchor::TopLeft,
        nalgebra_glm::vec2(15.0, 85.0),
        actions_props,
    );

    let instructions_props = TextProperties {
        font_size: 20.0,
        color: nalgebra_glm::vec4(0.7, 0.7, 0.7, 1.0),
        alignment: TextAlignment::Left,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    let instructions_text = spawn_hud_text_with_properties(
        world,
        "[SPACE] End Turn  [S] Speech  [P] Pause  [+/-] Speed",
        HudAnchor::TopLeft,
        nalgebra_glm::vec2(15.0, 115.0),
        instructions_props,
    );

    let speed_props = TextProperties {
        font_size: 20.0,
        color: nalgebra_glm::vec4(0.9, 0.9, 0.5, 1.0),
        alignment: TextAlignment::Left,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    let speed_text = spawn_hud_text_with_properties(
        world,
        "Speed: 1x",
        HudAnchor::TopLeft,
        nalgebra_glm::vec2(15.0, 140.0),
        speed_props,
    );

    GameHud {
        turn_text: Some(turn_text),
        faction_text: Some(faction_text),
        actions_text: Some(actions_text),
        instructions_text: Some(instructions_text),
        speed_text: Some(speed_text),
    }
}

pub fn despawn_game_hud(hud: &mut GameHud, world: &mut World) {
    if let Some(entity) = hud.turn_text.take() {
        world.despawn_entities(&[entity]);
    }
    if let Some(entity) = hud.faction_text.take() {
        world.despawn_entities(&[entity]);
    }
    if let Some(entity) = hud.actions_text.take() {
        world.despawn_entities(&[entity]);
    }
    if let Some(entity) = hud.instructions_text.take() {
        world.despawn_entities(&[entity]);
    }
    if let Some(entity) = hud.speed_text.take() {
        world.despawn_entities(&[entity]);
    }
}

pub fn update_game_hud(
    hud: &GameHud,
    game_world: &GameWorld,
    world: &mut World,
    player_faction: Faction,
) {
    let is_player_turn = game_world.resources.current_faction == player_faction;

    if let Some(turn_entity) = hud.turn_text
        && let Some(text_index) = world.get_hud_text(turn_entity).map(|t| t.text_index)
    {
        world.resources.text_cache.set_text(
            text_index,
            format!("Turn {}", game_world.resources.turn_number),
        );
        if let Some(hud_text) = world.get_hud_text_mut(turn_entity) {
            hud_text.dirty = true;
        }
    }

    if let Some(faction_entity) = hud.faction_text {
        let faction = game_world.resources.current_faction;
        let name = faction_name(faction);
        let color = faction_color(faction);

        if let Some(text_index) = world.get_hud_text(faction_entity).map(|t| t.text_index) {
            world
                .resources
                .text_cache
                .set_text(text_index, name.to_string());
        }
        if let Some(hud_text) = world.get_hud_text_mut(faction_entity) {
            hud_text.properties.color = nalgebra_glm::vec4(color[0], color[1], color[2], color[3]);
            hud_text.dirty = true;
        }
    }

    if let Some(actions_entity) = hud.actions_text
        && let Some(text_index) = world.get_hud_text(actions_entity).map(|t| t.text_index)
    {
        world.resources.text_cache.set_text(
            text_index,
            format!("Actions: {}", game_world.resources.actions_remaining),
        );
        if let Some(hud_text) = world.get_hud_text_mut(actions_entity) {
            hud_text.dirty = true;
        }
    }

    if let Some(instructions_entity) = hud.instructions_text
        && let Some(text_index) = world
            .get_hud_text(instructions_entity)
            .map(|t| t.text_index)
    {
        let instructions = if is_player_turn {
            "[SPACE] End Turn  [S] Speech  [P] Pause  [+/-] Speed"
        } else {
            "[P] Pause  [+/-] Speed"
        };
        world
            .resources
            .text_cache
            .set_text(text_index, instructions.to_string());
        if let Some(hud_text) = world.get_hud_text_mut(instructions_entity) {
            hud_text.dirty = true;
        }
    }

    if let Some(speed_entity) = hud.speed_text
        && let Some(text_index) = world.get_hud_text(speed_entity).map(|t| t.text_index)
    {
        let speed = game_world.resources.game_speed;
        let speed_text = if speed >= 1.0 {
            format!("Speed: {}x", speed as i32)
        } else {
            format!("Speed: {:.2}x", speed)
        };
        world.resources.text_cache.set_text(text_index, speed_text);
        if let Some(hud_text) = world.get_hud_text_mut(speed_entity) {
            hud_text.dirty = true;
        }
    }
}
