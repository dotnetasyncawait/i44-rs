pub const NAME: &'_ str = "Code";

use crate::input::{mods::Mods, keys::Key, hotkey::{Hotkey, Hotkey::*}};

// hotkeys

pub fn open_settings() -> Hotkey { Remap(Mods::LC, Key::COMMA) }
pub fn fold() -> Hotkey { Remap(Mods::LCS, Key::LBRACE) }
pub fn unfold() -> Hotkey { Remap(Mods::LCS, Key::RBRACE) }
pub fn fold_all() -> Hotkey { Remap(Mods::LSA, Key::NUM4) }
pub fn unfold_all() -> Hotkey { Remap(Mods::LSA, Key::NUM3) }
pub fn goto_bracket() -> Hotkey { Remap(Mods::LCS, Key::BSLASH) }
pub fn param_hints() -> Hotkey { Remap(Mods::LCS, Key::SPACE) }
pub fn close_editor() -> Hotkey { Remap(Mods::LC, Key::F4) }
pub fn reopen_last_closed_tab() -> Hotkey { Remap(Mods::LCS, Key::T) }
pub fn go_to_definition() -> Hotkey { Remap(Mods::NONE, Key::F12) }
pub fn go_to_impl() -> Hotkey { Remap(Mods::LC, Key::F12) }
pub fn show_or_focus_hover() -> Hotkey { Remap(Mods::LSA, Key::NUM5) }
pub fn comment_line() -> Hotkey { Remap(Mods::LC, Key::FSLASH) }
pub fn quick_fix() -> Hotkey { Remap(Mods::LC, Key::PERIOD) }
pub fn togg_breakpoint() -> Hotkey { Remap(Mods::NONE, Key::F9) }
pub fn next_tab() -> Hotkey { Remap(Mods::LC, Key::PG_DOWN) }
pub fn prev_tab() -> Hotkey { Remap(Mods::LC, Key::PG_UP) }
pub fn new_file() -> Hotkey { Remap(Mods::LA, Key::INSERT) }
pub fn copy_cursor_up() -> Hotkey { Remap(Mods::LCA, Key::UP) }
pub fn copy_cursor_down() -> Hotkey { Remap(Mods::LCA, Key::DOWN) }
pub fn copy_line_down() -> Hotkey { Remap(Mods::LC, Key::D) }
pub fn show_explorer() -> Hotkey { Remap(Mods::LCS, Key::E) }
pub fn debug() -> Hotkey { Remap(Mods::LCS, Key::D) }
pub fn terminal() -> Hotkey { Remap(Mods::LSA, Key::NUM2) }
pub fn togg_source_ctrl() -> Hotkey { Remap(Mods::LCS, Key::G) }
pub fn expand_selection() -> Hotkey { Remap(Mods::LSA, Key::RIGHT) }
pub fn shrink_selection() -> Hotkey { Remap(Mods::LSA, Key::LEFT) }
pub fn prev_member() -> Hotkey { Remap(Mods::LA, Key::UP) }
pub fn next_member() -> Hotkey { Remap(Mods::LA, Key::DOWN) }
// pub fn insert_line_above() -> Hotkey { Remap(Mods::LCS, Key::ENTER) }
pub fn insert_line_below() -> Hotkey { Remap(Mods::LC, Key::ENTER) }
pub fn to_tabs() -> Hotkey { Remap(Mods::LCS, Key::NUM4) }
pub fn to_spaces() -> Hotkey { Remap(Mods::LCS, Key::NUM7) }
pub fn scroll_term_up_by_line() -> Hotkey { Remap(Mods::LCA, Key::PG_UP) }
pub fn scroll_term_down_by_line() -> Hotkey { Remap(Mods::LCA, Key::PG_DOWN) }
pub fn scroll_term_up_by_page() -> Hotkey { Remap(Mods::LS, Key::PG_UP) }
pub fn scroll_term_down_by_page() -> Hotkey { Remap(Mods::LS, Key::PG_DOWN) }
pub fn stop_debugger() -> Hotkey { Remap(Mods::LS, Key::F5) }
pub fn restart_debugger() -> Hotkey { Remap(Mods::LCS, Key::F5) }
pub fn step_over() -> Hotkey { Remap(Mods::NONE, Key::F10) }
pub fn step_into() -> Hotkey { Remap(Mods::NONE, Key::F11) }
pub fn step_out() -> Hotkey { Remap(Mods::LS, Key::F11) }
