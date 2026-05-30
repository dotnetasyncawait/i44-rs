use std::fmt::{self, Debug, Formatter};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(u16);

impl Debug for Key {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "Key(0x{:04X})", self.0)
	}
}
