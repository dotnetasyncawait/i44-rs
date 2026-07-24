use crate::input::{mods::Mods, keys::Key, hotkey::{Hotkey, Hotkey::*}};

pub const NAME: &'_ str = "Discord";

// hotkeys

pub fn nav_to_curr_call() -> Hotkey { Remap(Mods::LCSA, Key::V) }
pub fn disconnect() -> Hotkey { Remap(Mods::NONE, Key::F22) }
pub fn tgl_member_list_or_vc_chat() -> Hotkey { Remap(Mods::LC, Key::U) }
pub fn upload_file() -> Hotkey { Remap(Mods::LCS, Key::U) }
