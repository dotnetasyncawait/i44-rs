use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS,
	KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput};
use super::{mods::Mods, keys::Key, extensions::InputExt, constants::{CALL_NEXT, CALL_NEXT_END}};

pub struct InputBuilder {
	buf: Vec<INPUT>
}

impl InputBuilder {
	pub fn with_capacity(cap: usize) -> Self {
		Self { buf: Vec::with_capacity(cap) }
	}
	
	pub fn key_down(mut self, key: Key) -> Self {
		self.buf.push(if key.is_mouse_key() {
			INPUT::mouse_down(key, CALL_NEXT)
		} else {
			INPUT::keybd_down(key, CALL_NEXT)
		});
		
		self
	}
	
	pub fn key_up(mut self, key: Key) -> Self {
		self.buf.push(if key.is_mouse_key() {
			INPUT::mouse_up(key, CALL_NEXT)
		} else {
			INPUT::keybd_up(key, CALL_NEXT)
		});
		
		self
	}
	
	pub fn key_up_if(self, key: Key, cond: bool) -> Self {
		if cond { self.key_up(key) } else { self }
	}
	
	pub fn mods_down(mut self, mods: Mods) -> Self {
		if !mods.is_none() {
			self.add_mods(mods, true, false);
		}
		self
	}
	
	pub fn mods_down_masked(mut self, mods: Mods, to_mask: bool) -> Self {
		if !mods.is_none() {
			self.add_mods(mods, true, to_mask);
		}
		self
	}
	
	pub fn mods_up(mut self, mods: Mods) -> Self {
		if !mods.is_none() {
			self.add_mods(mods, false, false);
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
			if mods.contains(Mods::LC) { self.buf.push(INPUT::new_keybd(Key::LCTRL.0, f,    CALL_NEXT)) };
			if mods.contains(Mods::RC) { self.buf.push(INPUT::new_keybd(Key::RCTRL.0, f_ex, CALL_NEXT)) };
		}
		
		if mods.contains(Mods::LS) { self.buf.push(INPUT::new_keybd(Key::LSHIFT.0, f,    CALL_NEXT)) };
		if mods.contains(Mods::LA) { self.buf.push(INPUT::new_keybd(Key::LALT.0,   f,    CALL_NEXT)) };
		if mods.contains(Mods::LW) { self.buf.push(INPUT::new_keybd(Key::LWIN.0,   f_ex, CALL_NEXT)) };
		if mods.contains(Mods::RS) { self.buf.push(INPUT::new_keybd(Key::RSHIFT.0, f,    CALL_NEXT)) };
		if mods.contains(Mods::RA) { self.buf.push(INPUT::new_keybd(Key::RALT.0,   f_ex, CALL_NEXT)) };
		if mods.contains(Mods::RW) { self.buf.push(INPUT::new_keybd(Key::RWIN.0,   f_ex, CALL_NEXT)) };
		
		if !pressed {
			if mods.contains(Mods::LC) { self.buf.push(INPUT::new_keybd(Key::LCTRL.0, f,    CALL_NEXT)) };
			if mods.contains(Mods::RC) { self.buf.push(INPUT::new_keybd(Key::RCTRL.0, f_ex, CALL_NEXT)) };
		}
		
		if to_mask { self.buf.push(INPUT::keybd_up(Key::LCTRL, CALL_NEXT)); }
	}
	
	pub fn build(mut self) -> Vec<INPUT> {
		let len = self.buf.len();
		debug_assert!(len != 0);
		
		let last = &mut self.buf[len - 1];
		match last.r#type {
			INPUT_KEYBOARD => last.Anonymous.ki.dwExtraInfo = CALL_NEXT_END,
			INPUT_MOUSE    => last.Anonymous.mi.dwExtraInfo = CALL_NEXT_END,
			_ => unreachable!()
		}
		
		self.buf.clone() // TODO: use Option<>.take()
	}
	
	pub fn send(self) {
		unsafe { SendInput(&self.buf, size_of::<INPUT>() as i32); }
	}
}