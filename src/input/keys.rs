use std::fmt::{self, Debug, Formatter};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub(super) u16);

impl Debug for Key {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "Key(0x{:04X})", self.0)
	}
}

impl Key {
	// letters
	pub const A: Key = Key(0x001E);
	pub const B: Key = Key(0x0030);
	
	// Numbers
	
	
	// Navigation
	
	
	// Symbols
	
	
	// Keypad
	
	
	// Function keys
	
	
	
	// mods
	pub const LCTRL:  Key = Key(0x001D);
	pub const LSHIFT: Key = Key(0x002A);
	pub const LALT:   Key = Key(0x0038);
	pub const LWIN:   Key = Key(0xE05B);
	pub const RCTRL:  Key = Key(0xE01D);
	pub const RSHIFT: Key = Key(0x0036);
	pub const RALT:   Key = Key(0xE038);
	pub const RWIN:   Key = Key(0xE05C);
	
	// Consumer
	
	
	// Misc
	
	
}