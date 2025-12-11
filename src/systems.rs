mod ai;
mod combat;
mod fireworks;
mod highlight;
mod hover;
mod input;
mod merge_popup;
mod movement;
mod range_lines;
mod reinforcement;
mod selection_visual;
mod speech;
mod tile_ownership;
mod turn;
mod unit;
mod unit_text;
mod valid_moves;
mod victory;

pub use ai::{ai_turn_system, build_turn_order};
pub use combat::resolve_combat;
pub use fireworks::{FireworkShell, spawn_capture_firework, update_firework_shells};
pub use highlight::{hover_outline_system, tile_highlight_system};
pub use hover::hover_system;
pub use input::input_system;
pub use merge_popup::{floating_popup_system, spawn_capture_popup, spawn_merge_popup};
pub use movement::movement_system;
pub use range_lines::range_lines_system;
pub use reinforcement::{PendingSpawn, reinforcement_system};
pub use selection_visual::selection_visual_system;
pub use speech::speech_system;
pub use tile_ownership::tile_ownership_system;
pub use turn::{can_end_turn, end_turn};
pub use unit::{
    UNIT_SELECTED_COLOR, UNIT_TEXT_HEIGHT_OFFSET, despawn_unit, move_unit_to, spawn_unit,
    unit_radius_for_soldiers, unit_visual_update_system,
};
pub use unit_text::unit_text_system;
pub use valid_moves::{calculate_valid_moves, find_path, valid_moves_system};
pub use victory::{GameResult, victory_system};
