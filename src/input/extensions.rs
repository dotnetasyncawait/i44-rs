use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_KEYBOARD,
	KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE};
use super::keys::Key;

pub trait InputExt {
	fn new_keybd(sc: u16, flags: KEYBD_EVENT_FLAGS, extra: usize) -> Self;
	fn keybd_down(key: Key, extra: usize) -> Self;
	fn keybd_up(key: Key, extra: usize) -> Self;
	fn keybd_with_flags(key: Key, flags: KEYBD_EVENT_FLAGS, extra: usize) -> Self;
}

impl InputExt for INPUT {
	fn new_keybd(sc: u16, flags: KEYBD_EVENT_FLAGS, extra: usize) -> Self {
		Self { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 {
			ki: KEYBDINPUT { wScan: sc, dwFlags: flags, dwExtraInfo: extra, ..Default::default() } }
		}
	}
	
	fn keybd_with_flags(key: Key, flags: KEYBD_EVENT_FLAGS, extra: usize) -> Self {
		Self::new_keybd(key.0, flags, extra)
	}
	
	fn keybd_down(key: Key, extra: usize) -> Self {
		let mut flags = KEYEVENTF_SCANCODE;
		if key.is_extended_key() {
			flags |= KEYEVENTF_EXTENDEDKEY;
		}
		Self::new_keybd(key.0, flags, extra)
	}
	
	fn keybd_up(key: Key, extra: usize) -> Self {
		let mut flags = KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP;
		if key.is_extended_key() {
			flags |= KEYEVENTF_EXTENDEDKEY;
		}
		Self::new_keybd(key.0, flags, extra)
	}
}