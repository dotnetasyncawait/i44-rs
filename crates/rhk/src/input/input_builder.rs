use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_EXTENDEDKEY,
	KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE};
use super::{mods::Mods, keys::Key, extensions::InputExt, constants::{CALL_NEXT, CALL_NEXT_END}};

pub struct InputBuilder {
	buf: Vec<INPUT>
}

impl InputBuilder {
	pub fn unicode(str: &str) -> Vec<INPUT> {
		let encoded: Vec<u16> = str.encode_utf16().collect();
		let len = encoded.len();
		
		let mut inputs: Vec<INPUT> = Vec::with_capacity(len);
		Self::fill_chars(encoded, &mut inputs);
		
		let last = &mut inputs[len - 1];
		last.Anonymous.ki.dwExtraInfo = CALL_NEXT_END;
		
		inputs
	}
	
	pub fn add_unicode(mut self, encoded: Vec<u16>) -> Self {
		Self::fill_chars(encoded, &mut self.buf);
		self
	}
	
	fn fill_chars(encoded: Vec<u16>, buf: &mut Vec<INPUT>) {
		let mut iter = encoded.into_iter();
		
		while let Some(ch) = iter.next() {
			if ch < 0xD800 {
				buf.push(INPUT::new_keybd(ch, KEYEVENTF_UNICODE, CALL_NEXT));
				buf.push(INPUT::new_keybd(ch, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, CALL_NEXT));
			} else {
				let low = iter.next().expect("must be valid surrogate pair");
				buf.push(INPUT::new_keybd(ch,  KEYEVENTF_UNICODE, CALL_NEXT));
				buf.push(INPUT::new_keybd(low, KEYEVENTF_UNICODE, CALL_NEXT));
				buf.push(INPUT::new_keybd(ch,  KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, CALL_NEXT));
				buf.push(INPUT::new_keybd(low, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, CALL_NEXT));
			}
		}
	}
	
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
	
	pub fn key_down_if(self, key: Key, cond: bool) -> Self {
		if cond { self.key_down(key) } else { self }
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
			self.add_mods(mods, true);
		}
		self
	}
	
	pub fn mods_up(mut self, mods: Mods) -> Self {
		if !mods.is_none() {
			self.add_mods(mods, false);
		}
		self
	}
	
	pub fn mods_up_masked(mut self, mods: Mods, should_mask: bool) -> Self {
		if !mods.is_none() {
			if should_mask {
				self.buf.push(INPUT::new_keybd(Key::LCTRL.0, KEYEVENTF_SCANCODE, CALL_NEXT));
				self.add_mods(mods, false);
				self.buf.push(INPUT::new_keybd(Key::LCTRL.0, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP, CALL_NEXT));
			} else {
				self.add_mods(mods, false);
			}
		}
		self
	}
	
	fn add_mods(&mut self, mods: Mods, pressed: bool) {
		if pressed {
			let f = KEYEVENTF_SCANCODE;
			let f_ex = f | KEYEVENTF_EXTENDEDKEY;
			
			if mods.has(Mods::LC) { self.buf.push(INPUT::new_keybd(Key::LCTRL.0,  f,    CALL_NEXT)) };
			if mods.has(Mods::RC) { self.buf.push(INPUT::new_keybd(Key::RCTRL.0,  f_ex, CALL_NEXT)) };
			if mods.has(Mods::LS) { self.buf.push(INPUT::new_keybd(Key::LSHIFT.0, f,    CALL_NEXT)) };
			if mods.has(Mods::RS) { self.buf.push(INPUT::new_keybd(Key::RSHIFT.0, f,    CALL_NEXT)) };
			if mods.has(Mods::LA) { self.buf.push(INPUT::new_keybd(Key::LALT.0,   f,    CALL_NEXT)) };
			if mods.has(Mods::RA) { self.buf.push(INPUT::new_keybd(Key::RALT.0,   f_ex, CALL_NEXT)) };
			if mods.has(Mods::LW) { self.buf.push(INPUT::new_keybd(Key::LWIN.0,   f_ex, CALL_NEXT)) };
			if mods.has(Mods::RW) { self.buf.push(INPUT::new_keybd(Key::RWIN.0,   f_ex, CALL_NEXT)) };
		} else {
			let f = KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP;
			let f_ex = f | KEYEVENTF_EXTENDEDKEY;
			
			if mods.has(Mods::RW) { self.buf.push(INPUT::new_keybd(Key::RWIN.0,   f_ex, CALL_NEXT)) };
			if mods.has(Mods::LW) { self.buf.push(INPUT::new_keybd(Key::LWIN.0,   f_ex, CALL_NEXT)) };
			if mods.has(Mods::RA) { self.buf.push(INPUT::new_keybd(Key::RALT.0,   f_ex, CALL_NEXT)) };
			if mods.has(Mods::LA) { self.buf.push(INPUT::new_keybd(Key::LALT.0,   f,    CALL_NEXT)) };
			if mods.has(Mods::RS) { self.buf.push(INPUT::new_keybd(Key::RSHIFT.0, f,    CALL_NEXT)) };
			if mods.has(Mods::LS) { self.buf.push(INPUT::new_keybd(Key::LSHIFT.0, f,    CALL_NEXT)) };
			if mods.has(Mods::RC) { self.buf.push(INPUT::new_keybd(Key::RCTRL.0,  f_ex, CALL_NEXT)) };
			if mods.has(Mods::LC) { self.buf.push(INPUT::new_keybd(Key::LCTRL.0,  f,    CALL_NEXT)) };
		}
	}
	
	pub fn build(mut self) -> Vec<INPUT> {
		let len = self.buf.len();
		
		debug_assert!(len != 0);
		debug_assert_eq!(len, self.buf.capacity());
		
		let last = &mut self.buf[len - 1];
		match last.r#type {
			INPUT_KEYBOARD => last.Anonymous.ki.dwExtraInfo = CALL_NEXT_END,
			INPUT_MOUSE    => last.Anonymous.mi.dwExtraInfo = CALL_NEXT_END,
			_ => unreachable!()
		}
		
		std::mem::take(&mut self.buf)
	}
}