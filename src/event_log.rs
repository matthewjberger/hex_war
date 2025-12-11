use crate::ecs::{Faction, faction_color, faction_name};
use nightshade::prelude::*;
use std::collections::VecDeque;

const MAX_LOG_ENTRIES: usize = 1000;
const VISIBLE_ENTRIES: usize = 8;
const LOG_FONT_SIZE: f32 = 16.0;
const LOG_LINE_HEIGHT: f32 = 20.0;
const LOG_PADDING: f32 = 10.0;
const FACTION_TAG_WIDTH: f32 = 85.0;
const LOG_WIDTH: f32 = 350.0;
const LOG_HEIGHT: f32 = VISIBLE_ENTRIES as f32 * LOG_LINE_HEIGHT + LOG_PADDING * 2.0;

#[derive(Clone)]
pub struct LogEntry {
    pub faction_tag: String,
    pub faction_color: [f32; 4],
    pub message: String,
}

pub struct LogLineEntities {
    pub faction_entity: Entity,
    pub message_entity: Entity,
}

pub struct EventLog {
    pub entries: VecDeque<LogEntry>,
    pub scroll_offset: usize,
    pub line_entities: Vec<LogLineEntities>,
}

pub fn event_log_new() -> EventLog {
    EventLog {
        entries: VecDeque::new(),
        scroll_offset: 0,
        line_entities: Vec::new(),
    }
}

fn event_log_add_entry(log: &mut EventLog, faction: Faction, message: String) {
    let faction_tag = format!("[{}]", faction_name(faction));
    let faction_color = faction_color(faction);
    log.entries.push_back(LogEntry {
        faction_tag,
        faction_color,
        message,
    });
    if log.entries.len() > MAX_LOG_ENTRIES {
        log.entries.pop_front();
        if log.scroll_offset > 0 {
            log.scroll_offset -= 1;
        }
    }
    log.scroll_offset = log.entries.len().saturating_sub(VISIBLE_ENTRIES);
}

pub fn event_log_add_combat(
    log: &mut EventLog,
    attacker_faction: Faction,
    defender_faction: Faction,
    attacker_survived: bool,
    defender_survived: bool,
) {
    let defender_name = faction_name(defender_faction);

    let message = if !defender_survived {
        format!("destroyed {} unit", defender_name)
    } else if !attacker_survived {
        format!("was repelled by {}", defender_name)
    } else {
        format!("attacked {}", defender_name)
    };

    event_log_add_entry(log, attacker_faction, message);
}

pub fn event_log_add_faction_eliminated(log: &mut EventLog, eliminated_faction: Faction) {
    event_log_add_entry(log, eliminated_faction, "has been eliminated!".to_string());
}

pub fn event_log_add_reinforcement(
    log: &mut EventLog,
    faction: Faction,
    soldiers: i32,
    location: &str,
) {
    let message = format!("reinforced {} (+{})", location, soldiers);
    event_log_add_entry(log, faction, message);
}

pub fn event_log_add_turn_start(log: &mut EventLog, turn: u32, faction: Faction) {
    let message = format!("Turn {} begins", turn);
    event_log_add_entry(log, faction, message);
}

pub fn event_log_add_speech(log: &mut EventLog, faction: Faction) {
    event_log_add_entry(log, faction, "gave an inspiring speech".to_string());
}

pub fn spawn_event_log_ui(world: &mut World, log: &mut EventLog) {
    let faction_props = TextProperties {
        font_size: LOG_FONT_SIZE,
        color: nalgebra_glm::vec4(1.0, 1.0, 1.0, 1.0),
        alignment: TextAlignment::Left,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    let message_props = TextProperties {
        font_size: LOG_FONT_SIZE,
        color: nalgebra_glm::vec4(1.0, 1.0, 1.0, 1.0),
        alignment: TextAlignment::Left,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    for index in 0..VISIBLE_ENTRIES {
        let y_offset = -(LOG_PADDING
            + (VISIBLE_ENTRIES - 1 - index) as f32 * LOG_LINE_HEIGHT
            + LOG_LINE_HEIGHT);

        let faction_entity = spawn_hud_text_with_properties(
            world,
            "",
            HudAnchor::BottomLeft,
            nalgebra_glm::vec2(LOG_PADDING, y_offset),
            faction_props.clone(),
        );

        let message_entity = spawn_hud_text_with_properties(
            world,
            "",
            HudAnchor::BottomLeft,
            nalgebra_glm::vec2(LOG_PADDING + FACTION_TAG_WIDTH, y_offset),
            message_props.clone(),
        );

        log.line_entities.push(LogLineEntities {
            faction_entity,
            message_entity,
        });
    }
}

pub fn despawn_event_log_ui(world: &mut World, log: &mut EventLog) {
    for line in log.line_entities.drain(..) {
        world.despawn_entities(&[line.faction_entity, line.message_entity]);
    }
}

pub fn update_event_log_ui(world: &mut World, log: &EventLog) {
    let start_index = log.scroll_offset;
    let entries_to_show: Vec<_> = log
        .entries
        .iter()
        .skip(start_index)
        .take(VISIBLE_ENTRIES)
        .cloned()
        .collect();

    for (slot_index, line) in log.line_entities.iter().enumerate() {
        if let Some(entry) = entries_to_show.get(slot_index) {
            if let Some(text_index) = world
                .get_hud_text(line.faction_entity)
                .map(|t| t.text_index)
            {
                world
                    .resources
                    .text_cache
                    .set_text(text_index, entry.faction_tag.clone());
            }
            if let Some(hud_text) = world.get_hud_text_mut(line.faction_entity) {
                hud_text.properties.color = nalgebra_glm::vec4(
                    entry.faction_color[0],
                    entry.faction_color[1],
                    entry.faction_color[2],
                    entry.faction_color[3],
                );
                hud_text.dirty = true;
            }

            if let Some(text_index) = world
                .get_hud_text(line.message_entity)
                .map(|t| t.text_index)
            {
                world
                    .resources
                    .text_cache
                    .set_text(text_index, entry.message.clone());
            }
            if let Some(hud_text) = world.get_hud_text_mut(line.message_entity) {
                hud_text.properties.color = nalgebra_glm::vec4(1.0, 1.0, 1.0, 1.0);
                hud_text.dirty = true;
            }
        } else {
            if let Some(text_index) = world
                .get_hud_text(line.faction_entity)
                .map(|t| t.text_index)
            {
                world
                    .resources
                    .text_cache
                    .set_text(text_index, String::new());
            }
            if let Some(hud_text) = world.get_hud_text_mut(line.faction_entity) {
                hud_text.dirty = true;
            }

            if let Some(text_index) = world
                .get_hud_text(line.message_entity)
                .map(|t| t.text_index)
            {
                world
                    .resources
                    .text_cache
                    .set_text(text_index, String::new());
            }
            if let Some(hud_text) = world.get_hud_text_mut(line.message_entity) {
                hud_text.dirty = true;
            }
        }
    }
}

pub fn event_log_scroll_system(log: &mut EventLog, world: &mut World) {
    let mouse_pos = world.resources.input.mouse.position;
    let screen_height = world
        .resources
        .window
        .handle
        .as_ref()
        .map(|h| h.inner_size().height as f32)
        .unwrap_or(600.0);

    let log_left = 0.0;
    let log_right = LOG_WIDTH;
    let log_bottom = screen_height;
    let log_top = screen_height - LOG_HEIGHT;

    let in_log_area = mouse_pos.x >= log_left
        && mouse_pos.x <= log_right
        && mouse_pos.y >= log_top
        && mouse_pos.y <= log_bottom;

    world.resources.user_interface.hud_wants_pointer = in_log_area;

    if !in_log_area {
        return;
    }

    if !world
        .resources
        .input
        .mouse
        .state
        .contains(MouseState::SCROLLED)
    {
        return;
    }

    let scroll_lines = -world.resources.input.mouse.wheel_delta.y.round() as i32;
    let max_scroll = log.entries.len().saturating_sub(VISIBLE_ENTRIES);

    if scroll_lines < 0 {
        log.scroll_offset = log
            .scroll_offset
            .saturating_sub(scroll_lines.unsigned_abs() as usize);
    } else {
        log.scroll_offset = (log.scroll_offset + scroll_lines as usize).min(max_scroll);
    }
}
