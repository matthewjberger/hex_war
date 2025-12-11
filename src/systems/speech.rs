use crate::constants::{MAX_MORALE, SPEECH_MORALE_BOOST};
use crate::ecs::{GameEvents, GameWorld, SpeechEvent, UNIT};

pub fn speech_system(game_world: &mut GameWorld, speech_requested: bool, events: &mut GameEvents) {
    if !speech_requested {
        return;
    }

    if game_world.resources.speech_used {
        return;
    }

    let current_faction = game_world.resources.current_faction;

    let faction_units: Vec<_> = game_world
        .query_entities(UNIT)
        .filter(|entity| {
            game_world
                .get_unit(*entity)
                .map(|unit| unit.faction == current_faction)
                .unwrap_or(false)
        })
        .collect();

    for entity in faction_units {
        if let Some(unit) = game_world.get_unit(entity) {
            let mut unit = *unit;
            unit.morale = (unit.morale + SPEECH_MORALE_BOOST).min(MAX_MORALE);
            game_world.set_unit(entity, unit);
        }
    }

    game_world.resources.speech_used = true;
    events.speech_events.push(SpeechEvent {
        faction: current_faction,
    });
}
