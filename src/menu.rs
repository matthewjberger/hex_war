use crate::ecs::{Difficulty, Faction, faction_color, faction_name};
use nightshade::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum MenuState {
    #[default]
    MainMenu,
    MapSetup,
    Playing,
    Paused,
    GameOver,
}

#[derive(Default)]
pub struct MenuData {
    pub state: MenuState,
    pub main_menu_buttons: Vec<MenuButton>,
    pub map_setup_buttons: Vec<MenuButton>,
    pub pause_menu_buttons: Vec<MenuButton>,
    pub game_over_buttons: Vec<MenuButton>,
    pub difficulty_buttons: Vec<MenuButton>,
    pub title_entity: Option<Entity>,
    pub subtitle_entity: Option<Entity>,
    pub difficulty_label_entity: Option<Entity>,
    pub game_over_winner: Option<Faction>,
    pub hovered_button_index: Option<usize>,
    pub hovered_difficulty_index: Option<usize>,
    pub selected_difficulty: Difficulty,
}

pub enum MenuAction {
    None,
    StartGame,
    EnterMapSetup,
    RegenerateMap,
    ResumeGame,
    ReturnToMainMenu,
    QuitGame,
    SetDifficulty(Difficulty),
}

pub struct MenuButton {
    pub entity: Entity,
    pub position: nalgebra_glm::Vec2,
    pub anchor: HudAnchor,
    pub width: f32,
    pub height: f32,
    pub base_color: nalgebra_glm::Vec4,
    pub hover_color: nalgebra_glm::Vec4,
}

pub fn despawn_menu_elements(menu: &mut MenuData, world: &mut World) {
    if let Some(entity) = menu.title_entity.take() {
        world.despawn_entities(&[entity]);
    }
    if let Some(entity) = menu.subtitle_entity.take() {
        world.despawn_entities(&[entity]);
    }
    if let Some(entity) = menu.difficulty_label_entity.take() {
        world.despawn_entities(&[entity]);
    }
    for button in menu.main_menu_buttons.drain(..) {
        world.despawn_entities(&[button.entity]);
    }
    for button in menu.map_setup_buttons.drain(..) {
        world.despawn_entities(&[button.entity]);
    }
    for button in menu.pause_menu_buttons.drain(..) {
        world.despawn_entities(&[button.entity]);
    }
    for button in menu.game_over_buttons.drain(..) {
        world.despawn_entities(&[button.entity]);
    }
    for button in menu.difficulty_buttons.drain(..) {
        world.despawn_entities(&[button.entity]);
    }
}

pub fn setup_main_menu(menu: &mut MenuData, world: &mut World) {
    despawn_menu_elements(menu, world);

    let title_props = TextProperties {
        font_size: 72.0,
        color: nalgebra_glm::vec4(1.0, 0.8, 0.2, 1.0),
        alignment: TextAlignment::Center,
        outline_width: 0.08,
        outline_color: nalgebra_glm::vec4(0.3, 0.1, 0.0, 1.0),
        ..Default::default()
    };

    menu.title_entity = Some(spawn_hud_text_with_properties(
        world,
        "HEX WAR",
        HudAnchor::Center,
        nalgebra_glm::vec2(0.0, -100.0),
        title_props,
    ));

    menu.main_menu_buttons.push(create_button(
        world,
        "NEW GAME",
        nalgebra_glm::vec2(0.0, 0.0),
        HudAnchor::Center,
        48.0,
    ));
    menu.main_menu_buttons.push(create_button(
        world,
        "QUIT",
        nalgebra_glm::vec2(0.0, 60.0),
        HudAnchor::Center,
        48.0,
    ));
}

pub fn setup_pause_menu(menu: &mut MenuData, world: &mut World) {
    despawn_menu_elements(menu, world);

    let title_props = TextProperties {
        font_size: 56.0,
        color: nalgebra_glm::vec4(1.0, 0.5, 0.2, 1.0),
        alignment: TextAlignment::Center,
        outline_width: 0.06,
        outline_color: nalgebra_glm::vec4(0.3, 0.1, 0.0, 1.0),
        ..Default::default()
    };

    menu.title_entity = Some(spawn_hud_text_with_properties(
        world,
        "PAUSED",
        HudAnchor::Center,
        nalgebra_glm::vec2(0.0, -100.0),
        title_props,
    ));

    menu.pause_menu_buttons.push(create_button(
        world,
        "RESUME",
        nalgebra_glm::vec2(0.0, -20.0),
        HudAnchor::Center,
        40.0,
    ));
    menu.pause_menu_buttons.push(create_button(
        world,
        "MAIN MENU",
        nalgebra_glm::vec2(0.0, 40.0),
        HudAnchor::Center,
        40.0,
    ));
}

pub fn main_menu_system(
    menu: &mut MenuData,
    world: &mut World,
    screen_width: f32,
    screen_height: f32,
) -> MenuAction {
    let mouse_x = world.resources.input.mouse.position.x;
    let mouse_y = world.resources.input.mouse.position.y;
    let clicked = world
        .resources
        .input
        .mouse
        .state
        .contains(MouseState::LEFT_JUST_RELEASED);

    menu.hovered_button_index = update_buttons_hover(
        &menu.main_menu_buttons,
        world,
        mouse_x,
        mouse_y,
        screen_width,
        screen_height,
        menu.hovered_button_index,
    );

    if clicked && let Some(index) = menu.hovered_button_index {
        return match index {
            0 => MenuAction::EnterMapSetup,
            1 => MenuAction::QuitGame,
            _ => MenuAction::None,
        };
    }

    MenuAction::None
}

pub fn setup_map_setup_menu(menu: &mut MenuData, world: &mut World) {
    despawn_menu_elements(menu, world);

    let title_props = TextProperties {
        font_size: 56.0,
        color: nalgebra_glm::vec4(0.8, 1.0, 0.8, 1.0),
        alignment: TextAlignment::Center,
        outline_width: 0.06,
        outline_color: nalgebra_glm::vec4(0.1, 0.3, 0.1, 1.0),
        ..Default::default()
    };

    menu.title_entity = Some(spawn_hud_text_with_properties(
        world,
        "MAP SETUP",
        HudAnchor::Center,
        nalgebra_glm::vec2(0.0, -150.0),
        title_props,
    ));

    let difficulty_label_props = TextProperties {
        font_size: 28.0,
        color: nalgebra_glm::vec4(0.7, 0.7, 0.7, 1.0),
        alignment: TextAlignment::Center,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    menu.difficulty_label_entity = Some(spawn_hud_text_with_properties(
        world,
        "DIFFICULTY",
        HudAnchor::Center,
        nalgebra_glm::vec2(0.0, -100.0),
        difficulty_label_props,
    ));

    menu.difficulty_buttons.push(create_difficulty_button(
        world,
        "EASY",
        nalgebra_glm::vec2(-120.0, -60.0),
        HudAnchor::Center,
        32.0,
        menu.selected_difficulty == Difficulty::Easy,
    ));
    menu.difficulty_buttons.push(create_difficulty_button(
        world,
        "NORMAL",
        nalgebra_glm::vec2(0.0, -60.0),
        HudAnchor::Center,
        32.0,
        menu.selected_difficulty == Difficulty::Normal,
    ));
    menu.difficulty_buttons.push(create_difficulty_button(
        world,
        "HARD",
        nalgebra_glm::vec2(120.0, -60.0),
        HudAnchor::Center,
        32.0,
        menu.selected_difficulty == Difficulty::Hard,
    ));

    menu.map_setup_buttons.push(create_button(
        world,
        "NEW MAP",
        nalgebra_glm::vec2(0.0, 0.0),
        HudAnchor::Center,
        48.0,
    ));
    menu.map_setup_buttons.push(create_button(
        world,
        "START GAME",
        nalgebra_glm::vec2(0.0, 60.0),
        HudAnchor::Center,
        48.0,
    ));
    menu.map_setup_buttons.push(create_button(
        world,
        "BACK",
        nalgebra_glm::vec2(0.0, 120.0),
        HudAnchor::Center,
        40.0,
    ));
}

pub fn map_setup_system(
    menu: &mut MenuData,
    world: &mut World,
    screen_width: f32,
    screen_height: f32,
) -> MenuAction {
    let mouse_x = world.resources.input.mouse.position.x;
    let mouse_y = world.resources.input.mouse.position.y;
    let clicked = world
        .resources
        .input
        .mouse
        .state
        .contains(MouseState::LEFT_JUST_RELEASED);

    menu.hovered_button_index = update_buttons_hover(
        &menu.map_setup_buttons,
        world,
        mouse_x,
        mouse_y,
        screen_width,
        screen_height,
        menu.hovered_button_index,
    );

    menu.hovered_difficulty_index = update_buttons_hover(
        &menu.difficulty_buttons,
        world,
        mouse_x,
        mouse_y,
        screen_width,
        screen_height,
        menu.hovered_difficulty_index,
    );

    if clicked {
        if let Some(index) = menu.hovered_difficulty_index {
            let difficulty = match index {
                0 => Difficulty::Easy,
                1 => Difficulty::Normal,
                2 => Difficulty::Hard,
                _ => menu.selected_difficulty,
            };
            return MenuAction::SetDifficulty(difficulty);
        }

        if let Some(index) = menu.hovered_button_index {
            return match index {
                0 => MenuAction::RegenerateMap,
                1 => MenuAction::StartGame,
                2 => MenuAction::ReturnToMainMenu,
                _ => MenuAction::None,
            };
        }
    }

    MenuAction::None
}

pub fn pause_menu_system(
    menu: &mut MenuData,
    world: &mut World,
    screen_width: f32,
    screen_height: f32,
) -> MenuAction {
    let mouse_x = world.resources.input.mouse.position.x;
    let mouse_y = world.resources.input.mouse.position.y;
    let clicked = world
        .resources
        .input
        .mouse
        .state
        .contains(MouseState::LEFT_JUST_RELEASED);

    menu.hovered_button_index = update_buttons_hover(
        &menu.pause_menu_buttons,
        world,
        mouse_x,
        mouse_y,
        screen_width,
        screen_height,
        menu.hovered_button_index,
    );

    if clicked && let Some(index) = menu.hovered_button_index {
        return match index {
            0 => MenuAction::ResumeGame,
            1 => MenuAction::ReturnToMainMenu,
            _ => MenuAction::None,
        };
    }

    MenuAction::None
}

fn create_button(
    world: &mut World,
    label: &str,
    position: nalgebra_glm::Vec2,
    anchor: HudAnchor,
    font_size: f32,
) -> MenuButton {
    let base_color = nalgebra_glm::vec4(0.8, 0.8, 0.8, 1.0);
    let hover_color = nalgebra_glm::vec4(1.0, 0.9, 0.3, 1.0);

    let props = TextProperties {
        font_size,
        color: base_color,
        alignment: TextAlignment::Center,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    let entity = spawn_hud_text_with_properties(world, label, anchor, position, props);

    let char_width = font_size * 0.55;
    let width = label.len() as f32 * char_width;
    let height = font_size * 1.2;

    MenuButton {
        entity,
        position,
        anchor,
        width,
        height,
        base_color,
        hover_color,
    }
}

fn create_difficulty_button(
    world: &mut World,
    label: &str,
    position: nalgebra_glm::Vec2,
    anchor: HudAnchor,
    font_size: f32,
    selected: bool,
) -> MenuButton {
    let base_color = if selected {
        nalgebra_glm::vec4(1.0, 0.9, 0.3, 1.0)
    } else {
        nalgebra_glm::vec4(0.6, 0.6, 0.6, 1.0)
    };
    let hover_color = nalgebra_glm::vec4(1.0, 0.9, 0.3, 1.0);

    let props = TextProperties {
        font_size,
        color: base_color,
        alignment: TextAlignment::Center,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    let entity = spawn_hud_text_with_properties(world, label, anchor, position, props);

    let char_width = font_size * 0.55;
    let width = label.len() as f32 * char_width;
    let height = font_size * 1.2;

    MenuButton {
        entity,
        position,
        anchor,
        width,
        height,
        base_color,
        hover_color,
    }
}

fn update_buttons_hover(
    buttons: &[MenuButton],
    world: &mut World,
    mouse_x: f32,
    mouse_y: f32,
    screen_width: f32,
    screen_height: f32,
    previous_hovered: Option<usize>,
) -> Option<usize> {
    let mut current_hovered = None;

    for (index, button) in buttons.iter().enumerate() {
        if is_point_in_bounds(button, mouse_x, mouse_y, screen_width, screen_height) {
            current_hovered = Some(index);
            break;
        }
    }

    if current_hovered != previous_hovered {
        if let Some(prev_index) = previous_hovered
            && let Some(button) = buttons.get(prev_index)
            && let Some(hud_text) = world.get_hud_text_mut(button.entity)
        {
            hud_text.properties.color = button.base_color;
            hud_text.dirty = true;
        }

        if let Some(curr_index) = current_hovered
            && let Some(button) = buttons.get(curr_index)
            && let Some(hud_text) = world.get_hud_text_mut(button.entity)
        {
            hud_text.properties.color = button.hover_color;
            hud_text.dirty = true;
        }
    }

    current_hovered
}

fn is_point_in_bounds(
    button: &MenuButton,
    mouse_x: f32,
    mouse_y: f32,
    screen_width: f32,
    screen_height: f32,
) -> bool {
    let base_x = match button.anchor {
        HudAnchor::TopLeft | HudAnchor::CenterLeft | HudAnchor::BottomLeft => 0.0,
        HudAnchor::TopCenter | HudAnchor::Center | HudAnchor::BottomCenter => screen_width * 0.5,
        HudAnchor::TopRight | HudAnchor::CenterRight | HudAnchor::BottomRight => screen_width,
    };

    let base_y = match button.anchor {
        HudAnchor::TopLeft | HudAnchor::TopCenter | HudAnchor::TopRight => 0.0,
        HudAnchor::CenterLeft | HudAnchor::Center | HudAnchor::CenterRight => screen_height * 0.5,
        HudAnchor::BottomLeft | HudAnchor::BottomCenter | HudAnchor::BottomRight => screen_height,
    };

    let screen_x = base_x + button.position.x;
    let screen_y = base_y + button.position.y;

    let left = screen_x - button.width * 0.5;
    let right = screen_x + button.width * 0.5;
    let top = screen_y - button.height * 0.5;
    let bottom = screen_y + button.height * 0.5;

    mouse_x >= left && mouse_x <= right && mouse_y >= top && mouse_y <= bottom
}

pub fn setup_game_over_menu(
    menu: &mut MenuData,
    world: &mut World,
    winner: Faction,
    is_player_winner: bool,
) {
    despawn_menu_elements(menu, world);
    menu.game_over_winner = Some(winner);

    let (title_text, title_color) = if is_player_winner {
        ("VICTORY!", nalgebra_glm::vec4(1.0, 0.85, 0.2, 1.0))
    } else {
        ("DEFEAT", nalgebra_glm::vec4(0.8, 0.2, 0.2, 1.0))
    };

    let title_props = TextProperties {
        font_size: 72.0,
        color: title_color,
        alignment: TextAlignment::Center,
        outline_width: 0.08,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    menu.title_entity = Some(spawn_hud_text_with_properties(
        world,
        title_text,
        HudAnchor::Center,
        nalgebra_glm::vec2(0.0, -120.0),
        title_props,
    ));

    let name = faction_name(winner);

    let subtitle_text = if is_player_winner {
        format!("{} conquers all!", name)
    } else {
        format!("{} has conquered the world!", name)
    };

    let color = faction_color(winner);
    let subtitle_props = TextProperties {
        font_size: 36.0,
        color: nalgebra_glm::vec4(color[0], color[1], color[2], 1.0),
        alignment: TextAlignment::Center,
        outline_width: 0.05,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };

    menu.subtitle_entity = Some(spawn_hud_text_with_properties(
        world,
        &subtitle_text,
        HudAnchor::Center,
        nalgebra_glm::vec2(0.0, -60.0),
        subtitle_props,
    ));

    menu.game_over_buttons.push(create_button(
        world,
        "NEW GAME",
        nalgebra_glm::vec2(0.0, 20.0),
        HudAnchor::Center,
        48.0,
    ));
    menu.game_over_buttons.push(create_button(
        world,
        "MAIN MENU",
        nalgebra_glm::vec2(0.0, 80.0),
        HudAnchor::Center,
        40.0,
    ));
}

pub fn game_over_system(
    menu: &mut MenuData,
    world: &mut World,
    screen_width: f32,
    screen_height: f32,
) -> MenuAction {
    let mouse_x = world.resources.input.mouse.position.x;
    let mouse_y = world.resources.input.mouse.position.y;
    let clicked = world
        .resources
        .input
        .mouse
        .state
        .contains(MouseState::LEFT_JUST_RELEASED);

    menu.hovered_button_index = update_buttons_hover(
        &menu.game_over_buttons,
        world,
        mouse_x,
        mouse_y,
        screen_width,
        screen_height,
        menu.hovered_button_index,
    );

    if clicked && let Some(index) = menu.hovered_button_index {
        return match index {
            0 => MenuAction::EnterMapSetup,
            1 => MenuAction::ReturnToMainMenu,
            _ => MenuAction::None,
        };
    }

    MenuAction::None
}
