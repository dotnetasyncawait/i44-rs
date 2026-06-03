use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, KEYBD_EVENT_FLAGS,
	KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput};
use super::{mods::Mods, keys::Key, extensions::InputExt, constants::CALL_NEXT};

pub struct KeySender {
	buf: Vec<INPUT>
}

impl KeySender {
	pub fn with_capacity(cap: usize) -> Self {
		Self { buf: Vec::with_capacity(cap) }
	}
	
	pub fn send_key_down(key: Key) {
		let input = if key.is_mouse_key(){
			todo!()
		} else {
			INPUT::keybd_down(key, CALL_NEXT)
		};
		
		unsafe { SendInput(&[input], size_of::<INPUT>() as i32); }
	}
	
	pub fn key_down(mut self, key: Key) -> Self {
		self.buf.push(if key.is_mouse_key() {
			todo!()
		} else {
			INPUT::keybd_down(key, CALL_NEXT)
		});
		
		self
	}
	
	pub fn key_up(mut self, key: Key) -> Self {
		self.buf.push(if key.is_mouse_key() {
			todo!()
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
	
	pub fn send(self) {
		unsafe { SendInput(&self.buf, size_of::<INPUT>() as i32); }
	}
}