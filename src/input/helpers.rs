use super::keys::Key;

pub fn is_mouse_key(key: Key) -> bool { key.0 & 0x0200 != 0 }
pub fn is_extended_key(key: Key) -> bool { key.0 & 0xE000 == 0xE000 }