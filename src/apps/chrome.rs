use crate::input::{mods::Mods, keys::Key, hotkey::{Hotkey, Hotkey::*}};

pub const NAME: &'_ str = "chrome";

pub fn is_youtube(title: impl AsRef<str>) -> bool {
	title.as_ref().ends_with("- YouTube - Google Chrome")
}

// hotkeys

pub fn new_tab() -> Hotkey { Remap(Mods::LC, Key::T) }
pub fn close_tab() -> Hotkey { Remap(Mods::LC, Key::W) }
pub fn reopen_last_closed_tab() -> Hotkey { Remap(Mods::LCS, Key::T) }
pub fn reload_tab() -> Hotkey { Remap(Mods::LC, Key::R) }
pub fn reload_tab_ignore_cache() -> Hotkey { Remap(Mods::LCS, Key::R) }
pub fn next_tab() -> Hotkey { Remap(Mods::LC, Key::PG_DOWN) }
pub fn prev_tab() -> Hotkey { Remap(Mods::LC, Key::PG_UP) }
pub fn focus_on_addr_bar() -> Hotkey { Remap(Mods::LC, Key::L) }
pub fn open_home_page() -> Hotkey { Remap(Mods::LA, Key::HOME) }
pub fn jump_to_rightmost_tab() -> Hotkey { Remap(Mods::LC, Key::NUM9) }
pub fn tabs() -> Hotkey { Remap(Mods::LCS, Key::A) }
pub fn tgl_loop_mode() -> Hotkey { Remap(Mods::LA, Key::L) }
pub fn increase_playb_speed() -> Hotkey { Remap(Mods::LA, Key::NUM4) }
pub fn decrease_playb_speed() -> Hotkey { Remap(Mods::LA, Key::NUM5) }
pub fn default_playb_speed() -> Hotkey { Remap(Mods::LA, Key::NUM6) }
