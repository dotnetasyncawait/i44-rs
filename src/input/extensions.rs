use super::keys::Key;
use windows::Win32::UI::{
	Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT,
		KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_HWHEEL,
		MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
		MOUSEINPUT},
	WindowsAndMessaging::{WHEEL_DELTA, XBUTTON1, XBUTTON2}};

pub trait InputExt {
	fn new_keybd(sc: u16, flags: KEYBD_EVENT_FLAGS, extra: usize) -> Self;
	fn new_mouse(data: u32, flags: MOUSE_EVENT_FLAGS, extra: usize) -> Self;
	fn keybd_down(key: Key, extra: usize) -> Self;
	fn keybd_up(key: Key, extra: usize) -> Self;
	fn mouse_down(key: Key, extra: usize) -> Self;
	fn mouse_up(key: Key, extra: usize) -> Self;
}

impl InputExt for INPUT {
	fn new_keybd(sc: u16, flags: KEYBD_EVENT_FLAGS, extra: usize) -> Self {
		Self { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 {
			ki: KEYBDINPUT { wScan: sc, dwFlags: flags, dwExtraInfo: extra, ..Default::default() } }
		}
	}
	
	fn new_mouse(data: u32, flags: MOUSE_EVENT_FLAGS, extra: usize) -> Self {
		Self { r#type: INPUT_MOUSE, Anonymous: INPUT_0 {
			mi: MOUSEINPUT { mouseData: data, dwFlags: flags, dwExtraInfo: extra, ..Default::default() }
		}}
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

	fn mouse_down(key: Key, extra: usize) -> Self {
		let (flags, data) = get_mouse_data(key.0);
		Self::new_mouse(data, flags, extra)
	}
	
	fn mouse_up(key: Key, extra: usize) -> Self {
		debug_assert!(key.is_mouse_button(), "wheel do not have 'up' state");
		
		let (mut flags, data) = get_mouse_data(key.0);
		flags.0 <<= 1; // MOUSEEVENTF_<key>UP
		
		Self::new_mouse(data, flags, extra)
	}
}

fn get_mouse_data(key: u16) -> (MOUSE_EVENT_FLAGS, u32) {
	const DELTA: i32 = WHEEL_DELTA as i32;
	
	return match Key(key & 0x02FF) {
		Key::LBUTTON  => (MOUSEEVENTF_LEFTDOWN,   0),
		Key::RBUTTON  => (MOUSEEVENTF_RIGHTDOWN,  0),
		Key::MBUTTON  => (MOUSEEVENTF_MIDDLEDOWN, 0),
		Key::XBUTTON1 => (MOUSEEVENTF_XDOWN, XBUTTON1 as _),
		Key::XBUTTON2 => (MOUSEEVENTF_XDOWN, XBUTTON2 as _),
		
		Key::WH_UP    => (MOUSEEVENTF_WHEEL,  ( DELTA * get_wheel_mult(key)) as _),
		Key::WH_DOWN  => (MOUSEEVENTF_WHEEL,  (-DELTA * get_wheel_mult(key)) as _),
		Key::WH_LEFT  => (MOUSEEVENTF_HWHEEL, (-DELTA * get_wheel_mult(key)) as _),
		Key::WH_RIGHT => (MOUSEEVENTF_HWHEEL, ( DELTA * get_wheel_mult(key)) as _),
		
		_ => panic!("invalid mouse key: {key:?}")
	};
	
	fn get_wheel_mult(wheel: u16) -> i32 {
		(((wheel & 0xF000) >> 12) + 1) as _
	}
}