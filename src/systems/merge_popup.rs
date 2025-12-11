use crate::ecs::{FLOATING_POPUP, FloatingPopup, GameWorld, TileType};
use nightshade::prelude::*;

const POPUP_LIFETIME: f32 = 1.5;
const POPUP_FLOAT_SPEED: f32 = 50.0;

fn spawn_floating_popup(
    game_world: &mut GameWorld,
    world: &mut World,
    position: Vec3,
    text: &str,
    color: Vec4,
    font_size: f32,
) {
    let text_position = position + nalgebra_glm::vec3(0.0, 150.0, 0.0);

    let text_entity = spawn_3d_billboard_text_with_properties(
        world,
        text,
        text_position,
        TextProperties {
            font_size,
            color,
            alignment: TextAlignment::Center,
            outline_width: 0.15,
            outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
            smoothing: 0.15,
            ..Default::default()
        },
    );

    let game_entity = game_world.spawn_entities(FLOATING_POPUP, 1)[0];
    game_world.set_floating_popup(
        game_entity,
        FloatingPopup {
            text_entity,
            lifetime: 0.0,
        },
    );
}

pub fn spawn_merge_popup(
    game_world: &mut GameWorld,
    world: &mut World,
    position: Vec3,
    amount: i32,
) {
    let text = format!("+{}", amount);
    let color = nalgebra_glm::vec4(0.2, 1.0, 0.2, 1.0);
    spawn_floating_popup(game_world, world, position, &text, color, 15000.0);
}

pub fn spawn_capture_popup(
    game_world: &mut GameWorld,
    world: &mut World,
    position: Vec3,
    tile_type: TileType,
) {
    let (text, color, font_size) = match tile_type {
        TileType::Capital => ("CAPITAL!", nalgebra_glm::vec4(1.0, 0.85, 0.0, 1.0), 20000.0),
        TileType::City => ("City", nalgebra_glm::vec4(0.3, 0.8, 1.0, 1.0), 15000.0),
        TileType::Port => ("Port", nalgebra_glm::vec4(0.4, 0.7, 0.9, 1.0), 15000.0),
        _ => return,
    };
    spawn_floating_popup(game_world, world, position, text, color, font_size);
}

pub fn floating_popup_system(game_world: &mut GameWorld, world: &mut World, delta_time: f32) {
    let entities: Vec<_> = game_world.query_entities(FLOATING_POPUP).collect();
    let mut popups_to_remove = Vec::new();
    let game_speed = game_world.resources.game_speed;

    for entity in entities {
        let Some(mut popup) = game_world.get_floating_popup(entity).copied() else {
            continue;
        };

        popup.lifetime += delta_time * game_speed;

        if popup.lifetime > POPUP_LIFETIME {
            popups_to_remove.push((entity, popup.text_entity));
            continue;
        }

        game_world.set_floating_popup(entity, popup);

        if let Some(transform) = world.get_local_transform_mut(popup.text_entity) {
            transform.translation.y += delta_time * POPUP_FLOAT_SPEED * game_speed;
        }
        mark_local_transform_dirty(world, popup.text_entity);

        if let Some(text_component) = world.get_text_mut(popup.text_entity) {
            let alpha = (1.0 - (popup.lifetime / POPUP_LIFETIME)).max(0.0);
            text_component.properties.color.w = alpha;
            text_component.dirty = true;
        }
    }

    for (entity, text_entity) in popups_to_remove {
        world.queue_command(WorldCommand::DespawnRecursive {
            entity: text_entity,
        });
        game_world.despawn_entities(&[entity]);
    }
}
