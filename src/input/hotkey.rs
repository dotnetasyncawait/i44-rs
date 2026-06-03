use super::{mods::Mods, keys::Key, key_event::KeyEvent};

#[derive(Debug, Clone)]
pub enum Hotkey {
	Default,
	Suppress,
	Remap(Mods, Key),
	Unicode(&'static str),
	Action(fn(KeyEvent)),
}