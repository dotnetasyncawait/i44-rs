pub const NAME: &str = "Obsidian";

use crate::input::{mods::Mods, keys::Key, hotkey::{Hotkey, Hotkey::*}};

// hotkeys

pub fn fold_more() -> Hotkey { Remap(Mods::LCS, Key::LBRACE) }
pub fn fold_less() -> Hotkey { Remap(Mods::LCS, Key::RBRACE) }
pub fn fold_all_headings_and_lists() -> Hotkey { Remap(Mods::LCSA, Key::LBRACE) }
pub fn unfold_all_headings_and_lists() -> Hotkey { Remap(Mods::LCSA, Key::RBRACE) }
pub fn open_settings() -> Hotkey { Remap(Mods::LC, Key::COMMA) }
pub fn close_curr_tab() -> Hotkey { Remap(Mods::LC, Key::W) }
pub fn undo_closed_tab() -> Hotkey { Remap(Mods::LCS, Key::T) }
pub fn togg_reading_view() -> Hotkey { Remap(Mods::LC, Key::E) }
pub fn next_tab() -> Hotkey { Remap(Mods::LC, Key::PG_DOWN) }
pub fn prev_tab() -> Hotkey { Remap(Mods::LC, Key::PG_UP) }
pub fn explorer_focus() -> Hotkey { Remap(Mods::LA, Key::NUM1) }
pub fn show_outline() -> Hotkey { Remap(Mods::LA, Key::NUM3) }
pub fn find_prev() -> Hotkey { Remap(Mods::LS, Key::F3) }
pub fn find_next() -> Hotkey { Remap(Mods::NONE, Key::F3) }
pub fn navigate_back() -> Hotkey { Remap(Mods::LA, Key::LEFT) }
pub fn navigate_forward() -> Hotkey { Remap(Mods::LA, Key::RIGHT) }
pub fn move_line_up() -> Hotkey { Remap(Mods::LSA, Key::UP) }
pub fn move_line_down() -> Hotkey { Remap(Mods::LSA, Key::DOWN) }
