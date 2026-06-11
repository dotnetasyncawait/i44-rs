pub const NAME: &'_ str = "explorer";

use crate::input::{mods::Mods, keys::Key, hotkey::{Hotkey, Hotkey::*}};

// hotkeys

pub fn focus_on_addr_bar() -> Hotkey { Remap(Mods::LA, Key::D) }
pub fn close_tab() -> Hotkey { Remap(Mods::LC, Key::W) }
pub fn next_tab() -> Hotkey { Remap(Mods::LC, Key::TAB) }
pub fn prev_tab() -> Hotkey { Remap(Mods::LCS, Key::TAB) }
pub fn new_tab() -> Hotkey { Remap(Mods::LC, Key::T) }
