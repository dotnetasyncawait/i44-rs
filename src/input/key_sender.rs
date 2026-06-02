use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, KEYBD_EVENT_FLAGS,
	KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput};
use super::{mods::Mods, keys::Key, helpers, extensions::InputExt, constants::CALL_NEXT};

pub struct KeySender {
	buf: Vec<INPUT>
}

impl KeySender {
	pub fn with_capacity(cap: usize) -> Self {
		Self { buf: Vec::with_capacity(cap) }
	}
	
	pub fn key_down(mut self, key: Key) -> Self {
		self.buf.push(if helpers::is_mouse_key(key) {
			todo!()
		} else {
			INPUT::keybd_down(key, CALL_NEXT)
		});
		
		self
	}
	
	pub fn mods_down(mut self, mods: Mods) -> Self {
		if !mods.is_none() {
			self.add_mods(mods, true, false);
		}
		self
	}
	
	pub fn mods_up_masked(mut self, mods: Mods, to_mask: bool) -> Self {
		if !mods.is_none() {
			self.add_mods(mods, false, to_mask);
		}
		self
	}
	
	fn add_mods(&mut self, mods: Mods, pressed: bool, to_mask: bool) {
		let f = KEYEVENTF_SCANCODE | if pressed { KEYBD_EVENT_FLAGS(0) } else { KEYEVENTF_KEYUP };
		let f_ex = f | KEYEVENTF_EXTENDEDKEY;
		
		if to_mask { self.buf.push(INPUT::keybd_down(Key::LCTRL, CALL_NEXT)); }
		
		if pressed {
			if mods.contains(Mods::LC) { self.buf.push(INPUT::keybd_with_flags(Key::LCTRL, f,    CALL_NEXT)) };
			if mods.contains(Mods::RC) { self.buf.push(INPUT::keybd_with_flags(Key::RCTRL, f_ex, CALL_NEXT)) };
		}
		
		if mods.contains(Mods::LS) { self.buf.push(INPUT::keybd_with_flags(Key::LSHIFT, f,    CALL_NEXT)) };
		if mods.contains(Mods::LA) { self.buf.push(INPUT::keybd_with_flags(Key::LALT,   f,    CALL_NEXT)) };
		if mods.contains(Mods::LW) { self.buf.push(INPUT::keybd_with_flags(Key::LWIN,   f_ex, CALL_NEXT)) };
		if mods.contains(Mods::RS) { self.buf.push(INPUT::keybd_with_flags(Key::RSHIFT, f,    CALL_NEXT)) };
		if mods.contains(Mods::RA) { self.buf.push(INPUT::keybd_with_flags(Key::RALT,   f_ex, CALL_NEXT)) };
		if mods.contains(Mods::RW) { self.buf.push(INPUT::keybd_with_flags(Key::RWIN,   f_ex, CALL_NEXT)) };
		
		if !pressed {
			if mods.contains(Mods::LC) { self.buf.push(INPUT::keybd_with_flags(Key::LCTRL, f,    CALL_NEXT)) };
			if mods.contains(Mods::RC) { self.buf.push(INPUT::keybd_with_flags(Key::RCTRL, f_ex, CALL_NEXT)) };
		}
		
		if to_mask { self.buf.push(INPUT::keybd_up(Key::LCTRL, CALL_NEXT)); }
	}
	
	pub fn send(self) {
		unsafe { SendInput(&self.buf, size_of::<INPUT>() as i32) };
	}
}