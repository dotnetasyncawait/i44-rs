use crate::common::error::Error;
use super::{mods::Mods, keys::Key, key_event::KeyEvent};

#[derive(Debug, Clone)]
pub enum Hotkey {
	Default,
	Suppress,
	SuppressOnce,
	Remap(Mods, Key),
	Unicode(&'static str),
	Action(fn(KeyEvent) -> Result<(), Error>),
	ActionRepeat(fn() -> Result<(), Error>),
}

impl Hotkey {
	pub fn ok(self) -> Result<Self, Error> {
		Ok(self)
	}
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HotkeyHandler {
	pub func: fn() -> Result<Hotkey, Error>,
	pub exempt: bool,
}

impl HotkeyHandler {
	pub fn new(f: fn() -> Result<Hotkey, Error>, exempt: bool) -> Self {
		Self { func: f, exempt }
	}
}