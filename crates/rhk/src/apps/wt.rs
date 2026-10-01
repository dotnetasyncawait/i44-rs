use crate::input::{mods::Mods, keys::Key, hotkey::{Hotkey, Hotkey::*}};

pub const NAME: &'_ str = "WindowsTerminal";

// hotkeys

pub fn duplicate_tab() -> Hotkey { Remap(Mods::LCS, Key::D) }
pub fn new_tab() -> Hotkey { Remap(Mods::LCS, Key::T) }
pub fn close_pane() -> Hotkey { Remap(Mods::LCS, Key::W) }
pub fn next_tab() -> Hotkey { Remap(Mods::LC, Key::TAB) }
pub fn prev_tab() -> Hotkey { Remap(Mods::LCS, Key::TAB) }
pub fn open_settings() -> Hotkey { Remap(Mods::LC, Key::COMMA) }
pub fn switch_to_tab0() -> Hotkey { Remap(Mods::LCA, Key::NUM1) }
pub fn switch_to_tab1() -> Hotkey { Remap(Mods::LCA, Key::NUM2) }
pub fn switch_to_tab2() -> Hotkey { Remap(Mods::LCA, Key::NUM3) }
pub fn switch_to_tab3() -> Hotkey { Remap(Mods::LCA, Key::NUM4) }
pub fn switch_to_tab4() -> Hotkey { Remap(Mods::LCA, Key::NUM5) }
pub fn switch_to_tab5() -> Hotkey { Remap(Mods::LCA, Key::NUM6) }
pub fn switch_to_last_tab() -> Hotkey { Remap(Mods::LCA, Key::NUM9) }
pub fn scroll_up() -> Hotkey { Remap(Mods::LCS, Key::UP) }
pub fn scroll_down() -> Hotkey { Remap(Mods::LCS, Key::DOWN) }
pub fn scroll_page_up() -> Hotkey { Remap(Mods::LCS, Key::PG_UP) }
pub fn scroll_page_down() -> Hotkey { Remap(Mods::LCS, Key::PG_DOWN) }
