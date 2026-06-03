use super::keys::Key;

pub fn is_mouse_key(key: Key) -> bool { key.0 & 0x0200 != 0 }

pub fn is_mouse_wheel(key: Key) -> bool {
	const L: u16 = Key::WH_UP.0;
	const U: u16 = Key::WH_RIGHT.0;
	matches!(key.0, L..=U)
}

pub fn is_mouse_button(key: Key) -> bool {
	const L: u16 = Key::LBUTTON.0;
	const U: u16 = Key::XBUTTON2.0;
	matches!(key.0, L..=U)
}

pub fn is_extended_key(key: Key) -> bool { key.0 & 0xE000 == 0xE000 }