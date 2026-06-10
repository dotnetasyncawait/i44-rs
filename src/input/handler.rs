use super::{hotkey::Hotkey, mods::Mods, keys::Key, input_builder::InputBuilder, extensions::InputExt};
use crate::common::error::Error;
use super::constants::{CALL_NEXT, CALL_NEXT_END, CACHED_EVENT};
use super::key_event::{KeyEvent, KeyEventNotifier};
use std::{collections::{HashMap, HashSet, hash_map::Entry}, ptr, sync::{mpsc, OnceLock, Mutex, MutexGuard}};
use std::thread::{self, JoinHandle};
use std::fmt::{self, Debug, Formatter};
use windows::core::Owned;
use windows::Win32::{Foundation::{LPARAM, LRESULT, WPARAM}, System::Threading::GetCurrentThreadId};
use windows::Win32::UI::{
	Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VK_TO_VSC_EX, VK_BROWSER_BACK, VK_LAUNCH_APP2, INPUT, SendInput},
	WindowsAndMessaging::{WH_KEYBOARD_LL, WH_MOUSE_LL, WM_QUIT, LLKHF_UP, LLKHF_EXTENDED, LLKHF_INJECTED, MSG,
		LLMHF_INJECTED, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
		WM_MOUSEWHEEL, WM_MOUSEHWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_XBUTTONDOWN, WM_XBUTTONUP, XBUTTON1, XBUTTON2,
		KBDLLHOOKSTRUCT, SetWindowsHookExW,GetMessageW, TranslateMessage, DispatchMessageW, CallNextHookEx,
		PostThreadMessageW}};

#[derive(Debug)]
pub struct Handler {
	hotkeys: HashMap<(Mods, Key), fn() -> Result<Hotkey, Error>>,
	suppressed: HashSet<Key>,
	curr_h: Option<CurrHotkey>,
	v_mods: Mods,
	send_count: u8,
	sender: mpsc::Sender<Vec<INPUT>>, // TODO: use Arc<Vec<INPUT>>?
}

#[derive(Debug)]
struct Worker(Option<JoinHandle<()>>, u32, i8);

#[derive(Debug, Clone, Copy)]
struct KeyMods { mods: Mods, key: Key }

impl KeyMods { 
	fn new(mods: Mods, key: Key) -> Self { Self { mods, key } }
}

enum CurrHotkey {
	Default(KeyMods),
	Remap(KeyMods, KeyMods),
	// Note: Unicode inputs are mostly short in length (usually 1 or 2 characters),
	// so cloning is ok (I guess...)
	Unicode(KeyMods, Vec<INPUT>),
	Action(KeyMods, KeyEventNotifier)
}
	
static HANDLER: OnceLock<Mutex<Handler>> = OnceLock::new();
static WORKER: OnceLock<Mutex<Worker>> = OnceLock::new();

const HANDLED: LRESULT = LRESULT(1);

pub fn wait() -> i8 {
	let Some(worker) = WORKER.get() else {
		unreachable!("should not be called before initialization");
	};
	
	let handle = worker.lock().unwrap().0.take();
	if let Some(h) = handle {
		h.join().unwrap();
		
		let mut h = HANDLER
			.get().expect("handler should be set if worker is")
			.lock().unwrap();
	
		let _ = std::mem::replace(&mut *h, Handler::new());
		
		worker.lock().unwrap().2 // ret_value
	} else {
		0
	}
}

pub fn exit(ret_value: i8) {
	if let Some(worker) = WORKER.get() {
		let mut w = worker.lock().unwrap();
		let thread_id = w.1;
		if thread_id != 0 {
			w.1 = 0;
			w.2 = ret_value;
			unsafe { PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0)).unwrap() };
		}
	}
}

impl Handler {
	pub fn new() -> Self {
		let (placeholder, _) = mpsc::channel();
		
		Self {
			hotkeys: HashMap::new(),
			suppressed: HashSet::new(),
			curr_h: None,
			v_mods: Mods::NONE,
			send_count: 0,
			sender: placeholder
		}
	}
	
	pub fn hotkey(&mut self, mods: Mods, key: Key, f: fn() -> Result<Hotkey, Error>) {
		match self.hotkeys.entry((mods, key)) {
			Entry::Occupied(o) => panic!("hotkey {:?} already exists", o.key()),
			Entry::Vacant(v) => v.insert_entry(f),
		};
	}
	
	pub fn start(mut self) {
		let (tx, rx) = mpsc::channel::<Vec<INPUT>>();
		let _ = thread::spawn(move || {
			for inputs in rx {
				unsafe { SendInput(&inputs, size_of::<INPUT>() as _) };	
			}
		});
		self.sender = tx;
		
		HANDLER.set(Mutex::new(self)).expect("handler should not be set");
		
		let (tx, rx) = mpsc::channel::<u32>();
		let handle = thread::spawn(move || Self::mq_handler(tx));
		
		let thread_id = rx.recv().unwrap();
		WORKER.set(Mutex::new(Worker(Some(handle), thread_id, 0))).expect("worker should not be set");
	}
	
	fn lock_handler() -> MutexGuard<'static, Handler> {
		HANDLER.get().unwrap().try_lock().expect("handler should always be available")
	}
	
	fn kb_get_key(s: &KBDLLHOOKSTRUCT) -> Option<(Key, bool)> {
		let mut sc = s.scanCode as u16;
		
		if s.flags.contains(LLKHF_INJECTED) {
			const VK_CONSUMER_BEGIN: u16 = VK_BROWSER_BACK.0;
			const VK_CONSUMER_END:   u16 = VK_LAUNCH_APP2.0;
			
			if sc == 0 && matches!(s.vkCode as u16, VK_CONSUMER_BEGIN..=VK_CONSUMER_END) {
				sc = unsafe { MapVirtualKeyW(s.vkCode, MAPVK_VK_TO_VSC_EX) } as u16;
				return Some((Key(sc), !s.flags.contains(LLKHF_UP)));
			}
			
			if s.dwExtraInfo != CACHED_EVENT {
				return None;
			}
		}
		
		if s.flags.contains(LLKHF_EXTENDED) && Key(sc) != Key::RSHIFT {
			sc |= 0xE000;
		}
		
		Some((Key(sc), !s.flags.contains(LLKHF_UP)))
	}
	
	fn ms_get_key(wparam: WPARAM, data: u32) -> (Key, bool) {
		match wparam.0 as u32 {
			WM_LBUTTONDOWN => (Key::LBUTTON, true),
			WM_LBUTTONUP   => (Key::LBUTTON, false),
			WM_RBUTTONDOWN => (Key::RBUTTON, true),
			WM_RBUTTONUP   => (Key::RBUTTON, false),
			WM_MBUTTONDOWN => (Key::MBUTTON, true),
			WM_MBUTTONUP   => (Key::MBUTTON, false),
			
			WM_XBUTTONDOWN if (data >> 16) == XBUTTON1 as _ => (Key::XBUTTON1, true),
			WM_XBUTTONDOWN if (data >> 16) == XBUTTON2 as _ => (Key::XBUTTON2, true),
			WM_XBUTTONUP   if (data >> 16) == XBUTTON1 as _ => (Key::XBUTTON1, false),
			WM_XBUTTONUP   if (data >> 16) == XBUTTON2 as _ => (Key::XBUTTON2, false),
			
			WM_MOUSEWHEEL  if ((data >> 16) as i16) > 0 => (Key::WH_UP, true),
			WM_MOUSEWHEEL  if ((data >> 16) as i16) < 0 => (Key::WH_DOWN, true),
			WM_MOUSEHWHEEL if ((data >> 16) as i16) < 0 => (Key::WH_LEFT, true),
			WM_MOUSEHWHEEL if ((data >> 16) as i16) > 0 => (Key::WH_RIGHT, true),
			
			_ => todo!("mouse wparam match should be exhaustive: 0x{:X}, {}", wparam.0, data)
		}
	}
	
	unsafe extern "system" fn ll_keybd_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
		if code < 0 {
			return call_next(code, wparam, lparam);
		}
		
		// IMPORTANT: drop locked handle before calling the next hook.
		// CallNextHookEx can pick up a message from the message queue and re-enter,
		// or execute Mouse hook (which will try to take the lock and panic).
		
		let s = unsafe { ptr::read(lparam.0 as *const KBDLLHOOKSTRUCT) };
		
		match s.dwExtraInfo {
			CALL_NEXT => return call_next(code, wparam, lparam),
			CALL_NEXT_END => {
				Self::lock_handler().send_count -= 1;
				return call_next(code, wparam, lparam);
			},
			_ => {}
		}
		
		let Some((key, pressed)) = Self::kb_get_key(&s) else {
			return HANDLED; // injected key
		};
		
		let mut h = Self::lock_handler();
		if h.send_count != 0 {
			if !pressed {
				h.sender.send(vec![INPUT::keybd_up(key, CACHED_EVENT)]).unwrap();
			}
			return HANDLED;
		}
		
		if Self::handle_kb(key, pressed, &mut h) {
			HANDLED
		} else {
			drop(h);
			call_next(code, wparam, lparam)
		}
	}
	
	unsafe extern "system" fn ll_mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
		if code < 0 || wparam.0 as u32 == WM_MOUSEMOVE {
			return call_next(code, wparam, lparam);
		}
		
		let s = unsafe { ptr::read(lparam.0 as *const MSLLHOOKSTRUCT) };
		
		match s.dwExtraInfo {
			CALL_NEXT => return call_next(code, wparam, lparam),
			CALL_NEXT_END => {
				Self::lock_handler().send_count -= 1;
				return call_next(code, wparam, lparam);
			},
			_ => {}
		}
		
		if s.flags & LLMHF_INJECTED != 0 && s.dwExtraInfo != CACHED_EVENT {
			return HANDLED;
		}
		
		let (key, pressed) = Self::ms_get_key(wparam, s.mouseData);
		
		let mut h = Self::lock_handler();
		if h.send_count != 0 {
			if !pressed {
				h.sender.send(vec![INPUT::mouse_up(key, CACHED_EVENT)]).unwrap();
			}
			return HANDLED;
		}
		
		if Self::handle_ms(key, pressed, &mut h) {
			HANDLED
		} else {
			call_next(code, wparam, lparam)
		}
	}
	
	fn handle_kb(key: Key, pressed: bool, h: &mut MutexGuard<'_, Handler>) -> bool {
		let mod_bit = Self::get_mod(key);
		
		if pressed {
			if Self::kb_key_down(key, mod_bit, h) {
				return true;
			}
			h.v_mods |= mod_bit;
		} else {
			if Self::kb_key_up(key, mod_bit, h) {
				return true;
			}
			h.v_mods &= !mod_bit;
		}
		
		false
	}
	
	fn handle_ms(key: Key, pressed: bool, h: &mut MutexGuard<'_, Handler>) -> bool {
		if pressed {
			if key.is_mouse_wheel() {
				Self::ms_wheel(key, h)
			} else {
				Self::kb_key_down(key, Mods::NONE, h)
			}
		} else {
			Self::kb_key_up(key, Mods::NONE, h)
		}
	}
	
	fn ms_wheel(key: Key, h: &mut MutexGuard<'_, Handler>) -> bool {
		if let Some(_) = &h.curr_h {
			return false; // TODO what?
		}
		
		let entry = KeyMods::new(h.v_mods, key);
		
		let Some(&f) = h.hotkeys.get(&(entry.mods, key)) else {
			return false;
		};
		
		let hotkey = match f() {
			Ok(hotkey) => hotkey,
			Err(err) => {
				println!("{err:?} ({entry:?})"); // TODO: display the error with a window
				return true;
			}
		};
		
		match hotkey {
			Hotkey::Default => false,
			Hotkey::Suppress => true,
			Hotkey::Remap(r_mods, r_key) => {
				let remap_mod_bit = Self::get_mod(r_key);
				let entry_mods = entry.mods & !(r_mods | remap_mod_bit);
				let remap_mods = r_mods & !entry.mods;
				
				let is_wheel = r_key.is_mouse_wheel();
				let should_mask = Self::should_mask(entry_mods);
				let size = ((entry_mods | remap_mods).count_ones() + (should_mask as u32 * 2)) * 2 + 1 + (!is_wheel as u32 * 1);
				
				let inputs = InputBuilder::with_capacity(size as _)
					.mods_up_masked(entry_mods, should_mask)
					.mods_down(remap_mods)
					.key_down(r_key)
					.key_up_if(r_key, !is_wheel)
					.mods_up(remap_mods)
					.mods_down_masked(entry_mods, should_mask)
					.build();
				
				h.send_count += 1;
				h.sender.send(inputs).unwrap();
				true
			},
			Hotkey::Unicode(str) => {
				let encoded = str.encode_utf16().collect::<Vec<u16>>();
				let entry_mods = entry.mods;
				let should_mask = Self::should_mask(entry_mods);
				let size = (entry_mods.count_ones() + (should_mask as u32 * 2) + encoded.len() as u32) * 2;
				
				if size != 0 {
					let inputs = InputBuilder::with_capacity(size as _)
						.mods_up_masked(entry_mods, should_mask)
						.add_unicode(encoded)
						.mods_down_masked(entry_mods, should_mask)
						.build();
					
					h.send_count += 1;
					h.sender.send(inputs).unwrap();
				}
				
				true
			},
			Hotkey::Action(action) => {
				let (event, notf) = KeyEvent::new();
				// Mouse wheel has only `down` state, so the event will always report 'up'.
				notf.notify();
				// Since there's no 'up' event for wheel-keys, we don't store the currently performing hotkey.
				// Hence, it will allow to run a new action before the previous one has completed.
				// It could be fixed by notifying back from the runner thread (once the action is run to completion),
				// but it would require adding a new field to the Handler/CurrHotkey::Action, specifically for this case.
				// Since actions on wheel keys are rare (not any for now), it should not be a problem.
				thread::spawn(move || {
					if let Err(err) = action(event) {
						println!("From action: {err:?}; ({entry:?})"); // TODO: display the error with a window
					}
				});
				
				true
			}
		}
	}
	
	fn kb_key_down(key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		if h.suppressed.contains(&key) {
			return true;
		}
		
		if let Some(curr_h) = &h.curr_h {
			return match curr_h {
				CurrHotkey::Default(entry) => Self::kb_default_repeat(*entry, key, mod_bit),
				CurrHotkey::Remap(entry, remap) => Self::kb_remap_repeat(*entry, *remap, key, mod_bit, h),
				CurrHotkey::Unicode(entry, inputs) => Self::kb_unicode_repeat(*entry, inputs.clone(), key, mod_bit, h),
				CurrHotkey::Action(entry, _) => Self::kb_action_repeat(*entry, key, mod_bit)
			};
		}
		
		if let Some(&f) = h.hotkeys.get(&(h.v_mods, key)) {
			Self::map_hotkey(f, KeyMods::new(h.v_mods, key), h)
		} else {
			false
		}
	}
	
	fn kb_key_up(key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		if h.suppressed.remove(&key) {
			return true;
		}
		
		match &h.curr_h {
			Some(curr_h) => match curr_h {
				CurrHotkey::Default(entry) => Self::kb_default_up(*entry, key, mod_bit, h),
				CurrHotkey::Remap(entry, remap) => Self::kb_remap_up(*entry, *remap, key, mod_bit, h),
				CurrHotkey::Unicode(entry, _) => Self::kb_unicode_up(*entry, key, mod_bit, h),
				CurrHotkey::Action(entry, notf) => Self::kb_action_up(*entry, notf.clone(), key, h)
			},
			None => false
		}
	}
	
	fn kb_default(entry: KeyMods, h: &mut MutexGuard<'_, Handler>) -> bool {
		h.curr_h = Some(CurrHotkey::Default(entry));
		false
	}
	
	fn kb_default_repeat(entry: KeyMods, key: Key, mod_bit: Mods) -> bool {
		if entry.key == key {
			false
		} else {
			// TODO: map hotkey if there's a match
			entry.mods.contains(mod_bit)
		}
	}
	
	fn kb_default_up(entry: KeyMods, key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		if entry.key == key || entry.mods.contains(mod_bit) {
			h.curr_h = None;
		}
		false
	}
	
	fn kb_suppress(key: Key, h: &mut MutexGuard<'_, Handler>) -> bool {
		h.suppressed.insert(key);
		true
	}
	
	fn kb_remap(entry: KeyMods, remap: KeyMods, h: &mut MutexGuard<'_, Handler>) -> bool {
		let remap_mod_bit = Self::get_mod(remap.key);
		let mods_to_release = entry.mods & !(remap.mods | remap_mod_bit);
		let mods_to_press = remap.mods & !entry.mods;
		
		let should_mask = Self::should_mask(mods_to_release);
		let size = (mods_to_release | mods_to_press).count_ones() + (should_mask as u32 * 2) + 1;
		
		let inputs = InputBuilder::with_capacity(size as _)
			.mods_up_masked(mods_to_release, should_mask)
			.mods_down(mods_to_press)
			.key_down(remap.key)
			.build();
		
		h.v_mods = remap.mods | remap_mod_bit;
		h.curr_h = Some(CurrHotkey::Remap(entry, remap));
		h.send_count += 1;
		h.sender.send(inputs).unwrap();
		
		true
	}
	
	fn kb_remap_up(entry: KeyMods, remap: KeyMods, key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		if key == entry.key {
			let key_to_release = remap.key;
			let mods_to_release = remap.mods & !entry.mods;
			let mods_to_restore = entry.mods & !remap.mods;
			
			h.curr_h = None;
			h.v_mods = (h.v_mods & !(mods_to_release | Self::get_mod(key_to_release))) | mods_to_restore;
			
			let should_mask = Self::should_mask(mods_to_restore);
			let is_wheel = key_to_release.is_mouse_wheel();
			let size = (mods_to_release | mods_to_restore).count_ones() + (should_mask as u32 * 2) + (!is_wheel as u32 * 1);
			
			if size != 0 {
				let inputs = InputBuilder::with_capacity(size as _)
					.key_up_if(key_to_release, !is_wheel)
					.mods_up(mods_to_release)
					.mods_down_masked(mods_to_restore, should_mask)
					.build();
				
				h.send_count += 1;
				h.sender.send(inputs).unwrap();
			}
			
			return true;
		}
		
		if entry.mods.contains(mod_bit) {
			let key_to_release = remap.key;
			let mods_to_release = remap.mods;
			
			h.curr_h = None;
			h.v_mods &= !(mods_to_release | Self::get_mod(key_to_release));
			
			Self::ignore_keys(entry.mods & !mod_bit, entry.key, h);
			
			let is_wheel = key_to_release.is_mouse_wheel();
			let size = mods_to_release.count_ones() + (!is_wheel as u32 * 1);
			
			if size != 0 {
				let inputs = InputBuilder::with_capacity(size as _)
					.key_up_if(key_to_release, !is_wheel)
					.mods_up(mods_to_release)
					.build();
				
				h.send_count += 1;
				h.sender.send(inputs).unwrap();
			}
			
			return true;
		}
		
		false
	}
	
	fn kb_remap_repeat(entry: KeyMods, remap: KeyMods, key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		if key == entry.key {
			let key_to_repeat = remap.key;
			
			if key_to_repeat.is_mouse_button() { // we don't repeat mouse buttons
				return true;
			}
			
			return if key == key_to_repeat {
				false
			} else {
				let input = if key_to_repeat.is_mouse_key() {
					INPUT::mouse_down(key_to_repeat, CALL_NEXT_END)
				} else {
					INPUT::keybd_down(key_to_repeat, CALL_NEXT_END)
				};
				
				h.send_count += 1;
				h.sender.send(vec![input]).unwrap();
				
				true
			};
		}
		
		if entry.mods.contains(mod_bit) { // suppress repeated entry mods (Qmk KeyOverrides issue)
			return true;
		}
		
		if !Self::get_mod(remap.key).is_none() { // key-to-mod remap
			return false;
		}
		
		let ph_mods = h.v_mods & !remap.mods | entry.mods;
		
		let Some(&f) = h.hotkeys.get(&(ph_mods, key)) else {
			return false;
		};
			
		let key_to_release = remap.key;
		let mods_to_release = remap.mods & !entry.mods;
		let mods_to_restore = entry.mods & !remap.mods;
		
		h.curr_h = None;
		h.v_mods = ph_mods;
		h.suppressed.insert(entry.key);
		
		let should_mask = Self::should_mask(mods_to_restore);
		let is_wheel = key_to_release.is_mouse_wheel();
		let size = (mods_to_release | mods_to_restore).count_ones() + (should_mask as u32 * 2) + (!is_wheel as u32 * 1);
		
		if size != 0 {
			let inputs = InputBuilder::with_capacity(size as _)
				.key_up_if(key_to_release, !is_wheel)
				.mods_up(mods_to_release)
				.mods_down_masked(mods_to_restore, should_mask)
				.build();
			
			h.send_count += 1;
			h.sender.send(inputs).unwrap();
		}
		
		Self::map_hotkey(f, KeyMods::new(h.v_mods, key), h)
	}
	
	fn kb_unicode(entry: KeyMods, str: &'static str, h: &mut MutexGuard<'_, Handler>) -> bool {
		if str.len() == 0 {
			return true; // same as Hotkey::Suppress
		}
		
		let inputs = InputBuilder::unicode(str);
		let mods_to_release = entry.mods;
		
		h.v_mods = Mods::NONE;
		h.curr_h = Some(CurrHotkey::Unicode(entry, inputs.clone()));
		
		if !mods_to_release.is_none() {
			let should_mask = Self::should_mask(mods_to_release);
			let size = mods_to_release.count_ones() + (should_mask as u32 * 2);
			
			let keys = InputBuilder::with_capacity(size as _)
				.mods_up_masked(mods_to_release, should_mask)
				.build();
			
			h.send_count += 1;
			h.sender.send(keys).unwrap();
		}
		
		h.send_count += 1;
		h.sender.send(inputs).unwrap();
		true
	}
	
	fn kb_unicode_up(entry: KeyMods, key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		let mods_to_restore;
		
		if key == entry.key {
			mods_to_restore = entry.mods;
		} else if entry.mods.contains(mod_bit) {
			mods_to_restore = entry.mods & !mod_bit;
			h.suppressed.insert(entry.key);
		} else {
			return false;
		}
		
		h.curr_h = None;
		h.v_mods |= mods_to_restore;
		
		if !mods_to_restore.is_none() {
			let should_mask = Self::should_mask(mods_to_restore);
			let size = mods_to_restore.count_ones() + (should_mask as u32 * 2);
			
			let inputs = InputBuilder::with_capacity(size as _)
				.mods_down_masked(mods_to_restore, should_mask)
				.build();
			
			h.send_count += 1;
			h.sender.send(inputs).unwrap();
		}
		
		true
	}
	
	fn kb_unicode_repeat(
		entry: KeyMods, inputs: Vec<INPUT>, key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool
	{
		if key == entry.key {
			h.send_count += 1;
			h.sender.send(inputs).unwrap();
			return true;
		}
		
		if entry.mods.contains(mod_bit) {
			return true;
		}
		
		let Some(&f) = h.hotkeys.get(&(h.v_mods | entry.mods, key)) else {
			return false;
		};
		
		let mods_to_restore = entry.mods;
		
		h.curr_h = None;
		h.v_mods |= mods_to_restore;
		h.suppressed.insert(entry.key);
		
		if !mods_to_restore.is_none() {
			let should_mask = Self::should_mask(mods_to_restore);
			let size = mods_to_restore.count_ones() + (should_mask as u32 * 2);
			
			let inputs = InputBuilder::with_capacity(size as _)
				.mods_down_masked(mods_to_restore, should_mask)
				.build();
			
			h.send_count += 1;
			h.sender.send(inputs).unwrap();
		}
		
		Self::map_hotkey(f, KeyMods::new(h.v_mods, key), h)
	}
	
	fn kb_action(entry: KeyMods, action: fn(KeyEvent) -> Result<(), Error>, h: &mut MutexGuard<'_, Handler>) -> bool {
		let (event, notf) = KeyEvent::new();
		h.curr_h = Some(CurrHotkey::Action(entry, notf));
		
		thread::spawn(move || {
			if let Err(err) = action(event) {
				println!("From action: {err:?}; ({entry:?})"); // TODO: display the error with a window
			}
		});
		
		true
	}
	
	fn kb_action_up(entry: KeyMods, notf: KeyEventNotifier, key: Key, h: &mut MutexGuard<'_, Handler>) -> bool {
		if key == entry.key {
			notf.notify();
			h.curr_h = None;
			true
		} else {
			false
		}
	}
	
	fn kb_action_repeat(entry: KeyMods, key: Key, mod_bit: Mods) -> bool {
		key == entry.key || entry.mods.contains(mod_bit)
	}
	
	fn map_hotkey(f: fn() -> Result<Hotkey, Error>, entry: KeyMods, h: &mut MutexGuard<'_, Handler>) -> bool {
		match f() {
			Ok(hotkey) => match hotkey {
				Hotkey::Default => Self::kb_default(entry, h),
				Hotkey::Suppress => Self::kb_suppress(entry.key, h),
				Hotkey::Remap(mods, key) => Self::kb_remap(entry, KeyMods::new(mods, key), h),
				Hotkey::Unicode(str) => Self::kb_unicode(entry, str, h),
				Hotkey::Action(action) => Self::kb_action(entry, action, h)
			},
			Err(err) => {
				println!("{err:?} ({entry:?})"); // TODO: display the error with a window
				true
			}
		}
	}
	
	fn ignore_keys(mods: Mods, key: Key, h: &mut MutexGuard<'_, Handler>) {
		if !mods.is_none() {
			if mods.contains(Mods::LC) { h.suppressed.insert(Key::LCTRL); }
			if mods.contains(Mods::LS) { h.suppressed.insert(Key::LSHIFT); }
			if mods.contains(Mods::LA) { h.suppressed.insert(Key::LALT); }
			if mods.contains(Mods::LW) { h.suppressed.insert(Key::LWIN); }
			if mods.contains(Mods::RC) { h.suppressed.insert(Key::RCTRL); }
			if mods.contains(Mods::RS) { h.suppressed.insert(Key::RSHIFT); }
			if mods.contains(Mods::RA) { h.suppressed.insert(Key::RALT); }
			if mods.contains(Mods::RW) { h.suppressed.insert(Key::RWIN); }
		}
		
		h.suppressed.insert(key);
	}
	
	fn get_mod(key: Key) -> Mods {
		match key {
			Key::LCTRL  => Mods::LC,
			Key::LSHIFT => Mods::LS,
			Key::LALT   => Mods::LA,
			Key::LWIN   => Mods::LW,
			Key::RCTRL  => Mods::RC,
			Key::RSHIFT => Mods::RS,
			Key::RALT   => Mods::RA,
			Key::RWIN   => Mods::RW,
			_ => Mods::NONE
		}
	}
	
	fn should_mask(mods: Mods) -> bool {
		mods.contains(Mods::LAW | Mods::RAW) && !mods.contains(Mods::LC | Mods::RC)
	}
	
	fn mq_handler(tx: mpsc::Sender<u32>) {
		let thread_id = unsafe { GetCurrentThreadId() };
		tx.send(thread_id).unwrap();
		drop(tx);
		
		let _keybd = unsafe { Owned::new(SetWindowsHookExW(WH_KEYBOARD_LL, Some(Self::ll_keybd_proc), None, 0).unwrap()) };
		let _mouse = unsafe { Owned::new(SetWindowsHookExW(WH_MOUSE_LL, Some(Self::ll_mouse_proc), None, 0).unwrap()) };
		
		let mut msg = MSG::default();
		
		loop {
			let res = unsafe { GetMessageW(&mut msg, None, 0, 0) };
			
			match res.0 {
				-1 => panic!("todo"),
				0 => break, // WM_QUIT
				_ => {
					_ = unsafe { TranslateMessage(&msg) };
					_ = unsafe { DispatchMessageW(&msg) };
				}
			}
		}
	}
}

fn call_next(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

impl Debug for CurrHotkey {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Default(entry) => f.debug_tuple("Default").field(entry).finish(),
			Self::Remap(entry, remap) => f.debug_tuple("Remap").field(entry).field(remap).finish(),
			Self::Unicode(entry, _) => f.debug_tuple("Unicode").field(entry).finish(), // TODO
			Self::Action(entry, notf) => f.debug_tuple("Action").field(entry).field(notf).finish(),
		}
	}
}