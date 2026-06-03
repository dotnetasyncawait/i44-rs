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
	
	
	
	// mouse keys
	
	pub const LBUTTON:  Key = Key(0x0200);
	pub const RBUTTON:  Key = Key(0x0201);
	pub const MBUTTON:  Key = Key(0x0202);
	pub const XBUTTON1: Key = Key(0x0203);
	pub const XBUTTON2: Key = Key(0x0204);
	pub const WH_UP:    Key = Key(0x0205);
	pub const WH_DOWN:  Key = Key(0x0206);
	pub const WH_LEFT:  Key = Key(0x0207);
	pub const WH_RIGHT: Key = Key(0x0208);
}