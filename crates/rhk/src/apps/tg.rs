use crate::input::{hotkey::Hotkey::{self, *}, mods::Mods, keys::Key};

pub const NAME: &str = "Telegram";

pub fn scroll_page_up() -> Hotkey { Remap(Mods::LC, Key::WH_UP) }
pub fn scroll_page_down() -> Hotkey { Remap(Mods::LC, Key::WH_DOWN) }