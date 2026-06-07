use crate::misc::error::Error;
use super::{mods::Mods, keys::Key, key_event::KeyEvent};

#[derive(Debug, Clone)]
pub enum Hotkey {
	Default,
	Suppress,
	Remap(Mods, Key),
	Unicode(&'static str),
	Action(fn(KeyEvent) -> Result<(), Error>),
}

impl Hotkey {
	pub fn ok(self) -> Result<Self, Error> {
		Ok(self)
	}
}