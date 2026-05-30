use super::{mods::Mods, keys::Key};

#[derive(Debug, Clone)]
pub enum Hotkey {
	Default,
	Suppress,
	Remap(Mods, Key),
	Unicode(&'static str),
	Action(fn(i32)),
}