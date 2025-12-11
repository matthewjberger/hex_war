mod camera;
mod constants;
mod ecs;
mod event_log;
mod hex;
mod hud;
mod instancing;
mod map;
mod map_generation;
mod menu;
mod prefabs;
mod rendering;
mod selection;
mod systems;
mod tiles;

use camera::{CameraBounds, calculate_camera_bounds, clamp_camera_to_bounds, reset_camera_to_map};
use constants::ACTIONS_PER_TURN;
use ecs::{Faction, GameEvents, GameWorld, TileType, UNIT};
use event_log::{
    EventLog, despawn_event_log_ui, event_log_add_combat, event_log_add_faction_eliminated,
    event_log_add_reinforcement, event_log_add_speech, event_log_add_turn_start, event_log_new,
    event_log_scroll_system, spawn_event_log_ui, update_event_log_ui,
};
use hex::hex_to_world_position;
use hud::{GameHud, despawn_game_hud, spawn_game_hud, update_game_hud};
use map_generation::{MapEntities, generate_game_map};
use menu::{MenuAction, MenuData, MenuState, game_over_system, map_setup_system};
use nightshade::ecs::prefab::Prefab;
use nightshade::prelude::*;
use prefabs::load_tile_prefabs;
use selection::clear_selection;
use std::collections::HashMap;
use systems::{
    FireworkShell, GameResult, PendingSpawn, ai_turn_system, build_turn_order, can_end_turn,
    despawn_unit, end_turn, floating_popup_system, hover_outline_system, hover_system,
    input_system, movement_system, range_lines_system, selection_visual_system,
    spawn_capture_firework, spawn_capture_popup, spawn_unit, speech_system, tile_highlight_system,
    tile_ownership_system, unit_text_system, unit_visual_update_system, update_firework_shells,
    valid_moves_system, victory_system,
};
use tiles::despawn_all_tiles;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    launch(HexWarGame::default())
}

fn spawn_fps_display(world: &mut World) -> Entity {
    let props = TextProperties {
        font_size: 24.0,
        color: nalgebra_glm::vec4(1.0, 1.0, 1.0, 1.0),
        alignment: TextAlignment::Right,
        outline_width: 0.02,
        outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
        ..Default::default()
    };
    spawn_hud_text_with_properties(
        world,
        "",
        HudAnchor::TopRight,
        nalgebra_glm::vec2(-10.0, 10.0),
        props,
    )
}

fn fps_display_system(world: &mut World, entity: Entity, visible: bool) {
    if !visible {
        return;
    }
    let Some(text_index) = world.get_hud_text(entity).map(|t| t.text_index) else {
        return;
    };
    let fps = world.resources.window.timing.frames_per_second;
    world
        .resources
        .text_cache
        .set_text(text_index, format!("FPS: {:.0}", fps));
    if let Some(hud_text) = world.get_hud_text_mut(entity) {
        hud_text.dirty = true;
    }
}

fn toggle_fps_display(world: &mut World, entity: Entity, visible: bool) {
    let Some(text_index) = world.get_hud_text(entity).map(|t| t.text_index) else {
        return;
    };
    if visible {
        let fps = world.resources.window.timing.frames_per_second;
        world
            .resources
            .text_cache
            .set_text(text_index, format!("FPS: {:.0}", fps));
    } else {
        world.resources.text_cache.set_text(text_index, "");
    }
    if let Some(hud_text) = world.get_hud_text_mut(entity) {
        hud_text.dirty = true;
    }
}

fn get_screen_size(world: &World) -> (f32, f32) {
    world
        .resources
        .window
        .handle
        .as_ref()
        .map(|handle| {
            let size = handle.inner_size();
            (size.width as f32, size.height as f32)
        })
        .unwrap_or((800.0, 600.0))
}

struct HexWarGame {
    game_world: GameWorld,
    game_events: GameEvents,
    map_entities: Option<MapEntities>,
    tile_prefabs: HashMap<TileType, Prefab>,
    menu: MenuData,
    game_hud: GameHud,
    event_log: EventLog,
    fps_entity: Option<Entity>,
    fps_visible: bool,
    sun_entity: Option<Entity>,
    speech_requested: bool,
    player_faction: Faction,
    pending_spawns: Vec<PendingSpawn>,
    camera_bounds: Option<CameraBounds>,
    firework_shells: Vec<FireworkShell>,
}

impl Default for HexWarGame {
    fn default() -> Self {
        Self {
            game_world: GameWorld::default(),
            game_events: GameEvents::default(),
            map_entities: None,
            tile_prefabs: HashMap::new(),
            menu: MenuData::default(),
            game_hud: GameHud::default(),
            event_log: event_log_new(),
            fps_entity: None,
            fps_visible: false,
            sun_entity: None,
            speech_requested: false,
            player_faction: Faction::default(),
            pending_spawns: Vec::new(),
            camera_bounds: None,
            firework_shells: Vec::new(),
        }
    }
}

fn game_reset_camera(game: &HexWarGame, world: &mut World) {
    reset_camera_to_map(
        world,
        game.game_world.resources.hex_width,
        game.game_world.resources.hex_depth,
        game.game_world.resources.map_params.map_width,
        game.game_world.resources.map_params.map_height,
    );
}

fn game_regenerate_map(game: &mut HexWarGame, world: &mut World) {
    game_cleanup_map(game, world);
    game.map_entities = Some(generate_game_map(
        &mut game.game_world,
        world,
        &game.tile_prefabs,
    ));
    game.camera_bounds = Some(calculate_camera_bounds(
        game.game_world.resources.hex_width,
        game.game_world.resources.hex_depth,
        game.game_world.resources.map_params.map_width,
        game.game_world.resources.map_params.map_height,
    ));
}

fn game_cleanup_map(game: &mut HexWarGame, world: &mut World) {
    if let Some(mut entities) = game.map_entities.take() {
        map_generation::despawn_map_entities(world, &mut entities);
    }

    let unit_entities: Vec<_> = game.game_world.query_entities(UNIT).collect();
    for entity in unit_entities {
        despawn_unit(&mut game.game_world, world, entity);
    }

    despawn_all_tiles(&mut game.game_world);
    clear_selection(&mut game.game_world);
    game.game_world.resources.hovered_tile = None;
    game.game_world.resources.previously_highlighted.clear();
    game.game_world.resources.previous_hovered_tile = None;
    game.game_world.resources.previous_selected_unit = None;
    game.game_world.resources.previous_valid_move_count = 0;
}

fn game_end_turn(game: &mut HexWarGame) {
    let transition = end_turn(&mut game.game_world, &mut game.game_events);
    event_log_add_turn_start(
        &mut game.event_log,
        transition.turn_number,
        transition.new_faction,
    );
    game.pending_spawns = transition.pending_spawns;
}

fn game_cleanup_game_world(game: &mut HexWarGame, world: &mut World) {
    world.resources.graphics.atmosphere = Atmosphere::None;

    if let Some(sun) = game.sun_entity.take() {
        world.queue_command(WorldCommand::DespawnRecursive { entity: sun });
    }

    despawn_game_hud(&mut game.game_hud, world);
    despawn_event_log_ui(world, &mut game.event_log);
    game_cleanup_map(game, world);
}

fn game_handle_menu_action(game: &mut HexWarGame, world: &mut World, action: MenuAction) {
    match action {
        MenuAction::None => {}
        MenuAction::EnterMapSetup => {
            game.menu.state = MenuState::MapSetup;
            menu::despawn_menu_elements(&mut game.menu, world);

            world.resources.graphics.atmosphere = Atmosphere::Nebula;
            game.sun_entity = Some(spawn_sun(world));
            game.map_entities = Some(generate_game_map(
                &mut game.game_world,
                world,
                &game.tile_prefabs,
            ));
            game.camera_bounds = Some(calculate_camera_bounds(
                game.game_world.resources.hex_width,
                game.game_world.resources.hex_depth,
                game.game_world.resources.map_params.map_width,
                game.game_world.resources.map_params.map_height,
            ));
            game_reset_camera(game, world);

            menu::setup_map_setup_menu(&mut game.menu, world);
        }
        MenuAction::RegenerateMap => {
            game_regenerate_map(game, world);
        }
        MenuAction::StartGame => {
            game.menu.state = MenuState::Playing;
            menu::despawn_menu_elements(&mut game.menu, world);

            game.game_world.resources.current_faction = Faction::Redosia;
            game.game_world.resources.actions_remaining = ACTIONS_PER_TURN;
            game.game_world.resources.turn_number = 1;
            game.game_world.resources.faction_eliminated = [false; 4];
            game.game_world.resources.game_speed = 1.0;
            game.game_world.resources.difficulty = game.menu.selected_difficulty;

            build_turn_order(&mut game.game_world);

            game.event_log = event_log_new();
            spawn_event_log_ui(world, &mut game.event_log);
            event_log_add_turn_start(&mut game.event_log, 1, Faction::Redosia);

            game.game_hud = spawn_game_hud(world);
        }
        MenuAction::ResumeGame => {
            game.menu.state = MenuState::Playing;
            menu::despawn_menu_elements(&mut game.menu, world);
            game.game_hud = spawn_game_hud(world);
        }
        MenuAction::ReturnToMainMenu => {
            game_cleanup_game_world(game, world);
            game.menu.state = MenuState::MainMenu;
            menu::setup_main_menu(&mut game.menu, world);
        }
        MenuAction::QuitGame => {
            world.resources.window.should_exit = true;
        }
        MenuAction::SetDifficulty(difficulty) => {
            game.menu.selected_difficulty = difficulty;
            menu::setup_map_setup_menu(&mut game.menu, world);
        }
    }
}

fn game_range_lines_entity(game: &HexWarGame) -> Option<Entity> {
    game.map_entities.as_ref().map(|e| e.range_lines_entity)
}

fn game_hover_outline_entity(game: &HexWarGame) -> Option<Entity> {
    game.map_entities.as_ref().map(|e| e.hover_outline_entity)
}

impl State for HexWarGame {
    fn title(&self) -> &str {
        "Hex War"
    }

    fn initialize(&mut self, world: &mut World) {
        world.resources.user_interface.enabled = false;
        world.resources.graphics.show_grid = false;
        world.resources.graphics.atmosphere = Atmosphere::None;

        if let Some(loaded) = load_tile_prefabs(world) {
            self.tile_prefabs = loaded.tile_prefabs;
            self.game_world.resources.hex_width = loaded.hex_width;
            self.game_world.resources.hex_depth = loaded.hex_depth;
        }

        let camera_entity = spawn_pan_orbit_camera(
            world,
            nalgebra_glm::vec3(0.0, 0.0, 0.0),
            4000.0,
            0.0,
            std::f32::consts::FRAC_PI_2 - 0.01,
            "Hex War Camera".to_string(),
        );
        world.resources.active_camera = Some(camera_entity);

        if let Some(pan_orbit) = world.get_pan_orbit_camera_mut(camera_entity) {
            pan_orbit.zoom_lower_limit = 500.0;
            pan_orbit.zoom_upper_limit = Some(6000.0);
            pan_orbit.pitch_lower_limit = 0.1;
        }

        self.fps_entity = Some(spawn_fps_display(world));
        menu::setup_main_menu(&mut self.menu, world);
    }

    fn run_systems(&mut self, world: &mut World) {
        let (screen_width, screen_height) = get_screen_size(world);

        match self.menu.state {
            MenuState::MainMenu => {
                let action =
                    menu::main_menu_system(&mut self.menu, world, screen_width, screen_height);
                game_handle_menu_action(self, world, action);
                return;
            }
            MenuState::MapSetup => {
                pan_orbit_camera_system(world);
                if let Some(bounds) = &self.camera_bounds {
                    clamp_camera_to_bounds(world, bounds);
                }
                let action = map_setup_system(&mut self.menu, world, screen_width, screen_height);
                game_handle_menu_action(self, world, action);
                return;
            }
            MenuState::Paused => {
                let action =
                    menu::pause_menu_system(&mut self.menu, world, screen_width, screen_height);
                game_handle_menu_action(self, world, action);
                return;
            }
            MenuState::GameOver => {
                pan_orbit_camera_system(world);
                if let Some(bounds) = &self.camera_bounds {
                    clamp_camera_to_bounds(world, bounds);
                }
                let action = game_over_system(&mut self.menu, world, screen_width, screen_height);
                game_handle_menu_action(self, world, action);
                return;
            }
            MenuState::Playing => {}
        }

        pan_orbit_camera_system(world);
        if let Some(bounds) = &self.camera_bounds {
            clamp_camera_to_bounds(world, bounds);
        }

        let hex_width = self.game_world.resources.hex_width;
        let hex_depth = self.game_world.resources.hex_depth;
        for pending in self.pending_spawns.drain(..) {
            spawn_unit(
                &mut self.game_world,
                world,
                pending.coord,
                hex_width,
                hex_depth,
                pending.faction,
                pending.soldiers,
            );
        }

        let delta_time = world.resources.window.timing.delta_time;
        update_particle_emitters(world, delta_time);
        update_firework_shells(&mut self.firework_shells, world, delta_time);
        movement_system(&mut self.game_world, world, delta_time);

        let is_ai_turn = self.game_world.resources.current_faction != self.player_faction;
        if is_ai_turn {
            let ai_done = ai_turn_system(
                &mut self.game_world,
                world,
                self.player_faction,
                &mut self.game_events,
            );
            if ai_done && can_end_turn(&self.game_world) {
                game_end_turn(self);
            }
        }

        if let Some(fps_entity) = self.fps_entity {
            fps_display_system(world, fps_entity, self.fps_visible);
        }

        let range_lines_entity = game_range_lines_entity(self);
        let hover_outline_entity = game_hover_outline_entity(self);

        if !is_ai_turn {
            hover_system(&mut self.game_world, world);
            input_system(&mut self.game_world, world, &mut self.game_events);
            speech_system(
                &mut self.game_world,
                self.speech_requested,
                &mut self.game_events,
            );
            self.speech_requested = false;
        }

        let captures = tile_ownership_system(&mut self.game_world);
        for capture in captures {
            let position = hex_to_world_position(
                capture.coord.column,
                capture.coord.row,
                hex_width,
                hex_depth,
            );
            spawn_capture_popup(&mut self.game_world, world, position, capture.tile_type);
            spawn_capture_firework(
                &mut self.firework_shells,
                world,
                position,
                capture.tile_type,
                capture.faction,
            );
        }

        selection_visual_system(&self.game_world, world);
        valid_moves_system(&mut self.game_world);
        range_lines_system(&mut self.game_world, world, range_lines_entity);

        if let Some(ref map_entities) = self.map_entities {
            tile_highlight_system(
                &mut self.game_world,
                world,
                &map_entities.instanced_tile_groups,
            );
        }
        hover_outline_system(&self.game_world, world, hover_outline_entity);
        unit_text_system(&self.game_world, world);
        unit_visual_update_system(&self.game_world, world);
        floating_popup_system(&mut self.game_world, world, delta_time);
        nightshade::ecs::text::systems::sync_text_meshes_system(world);
        update_game_hud(&self.game_hud, &self.game_world, world, self.player_faction);

        let game_result = victory_system(&mut self.game_world, world, &mut self.game_events);

        for event in self.game_events.combat_events.drain(..) {
            event_log_add_combat(
                &mut self.event_log,
                event.attacker_faction,
                event.defender_faction,
                event.attacker_survived,
                event.defender_survived,
            );
        }
        for event in self.game_events.speech_events.drain(..) {
            event_log_add_speech(&mut self.event_log, event.faction);
        }
        for event in self.game_events.reinforcement_events.drain(..) {
            event_log_add_reinforcement(
                &mut self.event_log,
                event.faction,
                event.soldiers,
                &event.location_name,
            );
        }
        for event in self.game_events.faction_eliminated_events.drain(..) {
            event_log_add_faction_eliminated(&mut self.event_log, event.faction);
        }

        event_log_scroll_system(&mut self.event_log, world);
        update_event_log_ui(world, &self.event_log);

        match game_result {
            GameResult::Victory(winner) => {
                let is_player_winner = winner == self.player_faction;
                despawn_game_hud(&mut self.game_hud, world);
                menu::setup_game_over_menu(&mut self.menu, world, winner, is_player_winner);
                self.menu.state = MenuState::GameOver;
            }
            GameResult::Ongoing => {}
        }

        self.game_world.step();
    }

    fn on_keyboard_input(&mut self, world: &mut World, key: KeyCode, state: KeyState) {
        if state != KeyState::Pressed {
            return;
        }

        match key {
            KeyCode::KeyP => match self.menu.state {
                MenuState::Playing => {
                    self.menu.state = MenuState::Paused;
                    despawn_game_hud(&mut self.game_hud, world);
                    menu::setup_pause_menu(&mut self.menu, world);
                }
                MenuState::Paused => {
                    self.menu.state = MenuState::Playing;
                    menu::despawn_menu_elements(&mut self.menu, world);
                    self.game_hud = spawn_game_hud(world);
                }
                MenuState::MainMenu | MenuState::MapSetup | MenuState::GameOver => {}
            },
            KeyCode::Space if self.menu.state == MenuState::Playing => {
                let is_player_turn =
                    self.game_world.resources.current_faction == self.player_faction;
                if is_player_turn {
                    game_end_turn(self);
                }
            }
            KeyCode::KeyS if self.menu.state == MenuState::Playing => {
                let is_player_turn =
                    self.game_world.resources.current_faction == self.player_faction;
                if is_player_turn {
                    self.speech_requested = true;
                }
            }
            KeyCode::Home | KeyCode::KeyC if self.menu.state == MenuState::Playing => {
                game_reset_camera(self, world);
            }
            KeyCode::KeyF => {
                self.fps_visible = !self.fps_visible;
                if let Some(fps_entity) = self.fps_entity {
                    toggle_fps_display(world, fps_entity, self.fps_visible);
                }
            }
            KeyCode::BracketRight | KeyCode::Equal if self.menu.state == MenuState::Playing => {
                let current = self.game_world.resources.game_speed;
                self.game_world.resources.game_speed = (current * 2.0).min(8.0);
            }
            KeyCode::BracketLeft | KeyCode::Minus if self.menu.state == MenuState::Playing => {
                let current = self.game_world.resources.game_speed;
                self.game_world.resources.game_speed = (current / 2.0).max(0.25);
            }
            _ => {}
        }
    }

    fn configure_render_graph(
        &mut self,
        graph: &mut RenderGraph<World>,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        resources: RenderResources,
    ) {
        let particle_pass = passes::ParticlePass::new(device, wgpu::TextureFormat::Rgba16Float);
        graph
            .pass(Box::new(particle_pass))
            .slot("color", resources.scene_color)
            .slot("depth", resources.depth);

        let (width, height) = (1920, 1080);
        let bloom_width = width / 2;
        let bloom_height = height / 2;

        let bloom_texture = graph
            .add_color_texture("bloom")
            .format(wgpu::TextureFormat::Rgba16Float)
            .size(bloom_width, bloom_height)
            .clear_color(wgpu::Color::BLACK)
            .transient();

        let bloom_pass = passes::BloomPass::new(device, width, height);
        graph
            .pass(Box::new(bloom_pass))
            .read("hdr", resources.scene_color)
            .write("bloom", bloom_texture);

        let postprocess_pass = passes::PostProcessPass::new(device, surface_format, 0.005);
        graph
            .pass(Box::new(postprocess_pass))
            .read("hdr", resources.scene_color)
            .read("bloom", bloom_texture)
            .write("output", resources.swapchain);
    }
}
