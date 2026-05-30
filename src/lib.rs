pub mod input;

use input::{handler::{self, Handler}, hotkey::Hotkey, mods::Mods, keys::Key};

pub struct App {
	h: Option<Handler>
}

impl App {
	pub fn new() -> Self {
		Self { h: Some(Handler::new()) }
	}
	
	pub fn hotkey(mut self, mods: Mods, key: Key, f: fn() -> Hotkey) -> Self {
		self.h.as_mut().unwrap().hotkey(mods, key, f);
		self
	}
	
	pub fn run(mut self) {
		self.h.take().unwrap().start();
		handler::wait();
	}
	
	pub fn exit() {
		handler::exit();
	}
}