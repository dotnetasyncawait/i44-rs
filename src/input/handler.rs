use super::{hotkey::Hotkey, mods::Mods, keys::Key, key_sender::KeySender, constants::CALL_NEXT, extensions::InputExt};
use super::key_event::{KeyEvent, KeyEventNotifier};
use std::{collections::{HashMap, HashSet}, ptr, sync::{mpsc, OnceLock, Mutex, MutexGuard, Arc}};
use std::thread::{self, JoinHandle};
use std::fmt::{self, Debug, Formatter};
use std::collections::hash_map::Entry;
use windows::core::Owned;
use windows::Win32::{Foundation::{LPARAM, LRESULT, WPARAM}, System::Threading::GetCurrentThreadId};
use windows::Win32::UI::{
	Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VK_TO_VSC_EX, VK_BROWSER_BACK, VK_LAUNCH_APP2, INPUT, SendInput,
		KEYEVENTF_UNICODE, KEYEVENTF_KEYUP },
	WindowsAndMessaging::{
		WH_KEYBOARD_LL, WM_QUIT, LLKHF_UP, LLKHF_EXTENDED, LLKHF_INJECTED, MSG, KBDLLHOOKSTRUCT,
		SetWindowsHookExW,GetMessageW, TranslateMessage, DispatchMessageW, CallNextHookEx, PostThreadMessageW}};


#[derive(Debug)]
pub struct Handler {
	hotkeys: HashMap<(Mods, Key), fn() -> Hotkey>,
	suppressed: HashSet<Key>,
	curr_h: Option<CurrHotkey>,
	v_mods: Mods,
}

#[derive(Debug)]
struct Worker(Option<JoinHandle<()>>, u32);

#[derive(Debug, Clone, Copy)]
struct KeyMods { mods: Mods, key: Key }

impl KeyMods { 
	fn new(mods: Mods, key: Key) -> Self { Self { mods, key } }
}

enum CurrHotkey {
	Default(KeyMods),
	Remap(KeyMods, KeyMods),
	Unicode(KeyMods, Arc<Vec<INPUT>>),
	Action(KeyMods, KeyEventNotifier)
}
	
static HANDLER: OnceLock<Mutex<Handler>> = OnceLock::new();
static WORKER: OnceLock<Mutex<Worker>> = OnceLock::new();

pub fn wait() {
	if let Some(worker) = WORKER.get() {
		let handle = worker.lock().unwrap().0.take();
		if let Some(h) = handle {
			h.join().unwrap();
		}
	}
}

pub fn exit() {
	if let Some(worker) = WORKER.get() {
		let thread_id = worker.lock().unwrap().1;
		unsafe { PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0)).unwrap() };
	}
}

impl Handler {
	pub fn new() -> Self {
		Self {
			hotkeys: HashMap::new(),
			suppressed: HashSet::new(),
			curr_h: None,
			v_mods: Mods::NONE,
		}
	}
	
	pub fn hotkey(&mut self, mods: Mods, key: Key, f: fn() -> Hotkey) {
		match self.hotkeys.entry((mods, key)) {
			Entry::Occupied(o) => panic!("hotkey {:?} already exists", o.key()),
			Entry::Vacant(v) => v.insert_entry(f),
		};
	}
	
	pub fn start(self) {
		HANDLER.set(Mutex::new(self)).expect("handler should not be set");
		
		let (tx, rx) = mpsc::channel::<u32>();
		let handle = thread::spawn(move || Self::mq_handler(tx));
		
		let thread_id = rx.recv().unwrap();
		WORKER.set(Mutex::new(Worker(Some(handle), thread_id))).expect("worker should not be set");
	}
	
	unsafe extern "system" fn ll_keybd_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
		if code < 0 {
			return unsafe { CallNextHookEx(None, code, wparam, lparam) };
		}
		
		let s = unsafe { ptr::read(lparam.0 as *const KBDLLHOOKSTRUCT) };
		
		match s.dwExtraInfo {
			CALL_NEXT => return unsafe { CallNextHookEx(None, code, wparam, lparam) },
			_ => {}
		};
		
		let mut sc = s.scanCode as u16;
		
		if s.flags.contains(LLKHF_INJECTED) {
			const VK_CONSUMER_BEGIN: u16 = VK_BROWSER_BACK.0;
			const VK_CONSUMER_END:   u16 = VK_LAUNCH_APP2.0;
			
			if sc == 0 && matches!(s.vkCode as u16, VK_CONSUMER_BEGIN..=VK_CONSUMER_END) {
				sc = unsafe { MapVirtualKeyW(s.vkCode, MAPVK_VK_TO_VSC_EX) } as u16;
			} else {
				return unsafe { CallNextHookEx(None, code, wparam, lparam) };
			}
		} else if s.flags.contains(LLKHF_EXTENDED) && Key(sc) != Key::RSHIFT {
			sc |= 0xE000;
		}
		
		let pressed = !s.flags.contains(LLKHF_UP);
		
		if Self::handle_kb(Key(sc), pressed) {
			LRESULT(1)
		} else {
			unsafe { CallNextHookEx(None, code, wparam, lparam) }
		}
	}
	
	fn handle_kb(key: Key, pressed: bool) -> bool {
		let mut h = HANDLER.get().unwrap().lock().unwrap();
		let mod_bit = Self::get_mod(key);
		
		if pressed {
			if Self::kb_key_down(key, mod_bit, &mut h) {
				return true;
			}
			h.v_mods |= mod_bit;
		} else {
			if Self::kb_key_up(key, mod_bit, &mut h) {
				return true;
			}
			h.v_mods &= !mod_bit;
		}
		
		false
	}
	
	fn kb_key_down(key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		if h.suppressed.contains(&key) {
			return true;
		}
		
		if let Some(curr_h) = &h.curr_h {
			return match curr_h {
				CurrHotkey::Default(entry) => Self::kb_default_repeat(*entry, key, mod_bit),
				CurrHotkey::Remap(entry, remap) => Self::kb_remap_repeat(*entry, *remap, key, mod_bit, h),
				CurrHotkey::Unicode(entry, inputs) => Self::kb_unicode_repeat(*entry, Arc::clone(inputs), key, mod_bit, h),
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
		h.curr_h = Some(CurrHotkey::Remap(entry, remap));
		
		let remap_mod_bit = Self::get_mod(remap.key);
		let mods_to_release = entry.mods & !(remap.mods | remap_mod_bit);
		let mods_to_press = remap.mods & !entry.mods;
		
		h.v_mods = remap.mods | remap_mod_bit;
		
		let should_mask = Self::should_mask(mods_to_release);
		let size = (mods_to_release | mods_to_press).count_ones() + (should_mask as u32 * 2) + 1;
		
		KeySender::with_capacity(size as usize)
			.mods_up_masked(mods_to_release, should_mask)
			.mods_down(mods_to_press)
			.key_down(remap.key)
			.send();
		
		true
	}
	
	fn kb_remap_up(entry: KeyMods, remap: KeyMods, key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		if key == entry.key {
			h.curr_h = None;
			
			let key_to_release = remap.key;
			let mods_to_release = remap.mods & !entry.mods;
			let mods_to_restore = entry.mods & !remap.mods;
			
			h.v_mods = (h.v_mods & !(mods_to_release | Self::get_mod(key_to_release))) | mods_to_restore;
			
			let should_mask = Self::should_mask(mods_to_restore);
			let is_wheel = key_to_release.is_mouse_wheel();
			let size = (mods_to_release | mods_to_restore).count_ones() + (should_mask as u32 * 2) + (!is_wheel as u32 * 1);
			
			if size != 0 {
				KeySender::with_capacity(size as usize)
					.key_up_if(key_to_release, !is_wheel)
					.mods_up(mods_to_release)
					.mods_down_masked(mods_to_restore, should_mask)
					.send();
			}
			
			return true;
		}
		
		if entry.mods.contains(mod_bit) {
			h.curr_h = None;
			
			let key_to_release = remap.key;
			let mods_to_release = remap.mods;
			
			h.v_mods &= !(mods_to_release | Self::get_mod(key_to_release));
			
			Self::ignore_keys(entry.mods & !mod_bit, entry.key, h);
			
			let is_wheel = key_to_release.is_mouse_wheel();
			let size = mods_to_release.count_ones() + (!is_wheel as u32 * 1);
			
			if size != 0 {
				KeySender::with_capacity(size as usize)
					.key_up_if(key_to_release, !is_wheel)
					.mods_up(mods_to_release)
					.send();
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
				KeySender::send_key_down(key_to_repeat);
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
			KeySender::with_capacity(size as usize)
				.key_up_if(key_to_release, !is_wheel)
				.mods_up(mods_to_release)
				.mods_down_masked(mods_to_restore, should_mask)
				.send();
		}
		
		Self::map_hotkey(f, KeyMods::new(h.v_mods, key), h)
	}
	
	fn kb_unicode(entry: KeyMods, str: &'static str, h: &mut MutexGuard<'_, Handler>) -> bool {
		let encoded: Vec<u16> = str.encode_utf16().collect();
		let mut inputs: Vec<INPUT> = Vec::with_capacity(encoded.len());
		let mut iter = encoded.into_iter();
		
		while let Some(ch) = iter.next() {
			if ch < 0xD800 {
				inputs.push(INPUT::new_keybd(ch, KEYEVENTF_UNICODE, CALL_NEXT));
				inputs.push(INPUT::new_keybd(ch, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, CALL_NEXT));
			} else {
				let low = iter.next().expect("must be valid surrogate pair");
				inputs.push(INPUT::new_keybd(ch,  KEYEVENTF_UNICODE, CALL_NEXT));
				inputs.push(INPUT::new_keybd(low, KEYEVENTF_UNICODE, CALL_NEXT));
				inputs.push(INPUT::new_keybd(ch,  KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, CALL_NEXT));
				inputs.push(INPUT::new_keybd(low, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, CALL_NEXT));
			}
		}
		
		let mods_to_release = entry.mods;
		let inputs = Arc::new(inputs);
		
		h.v_mods = Mods::NONE;
		h.curr_h = Some(CurrHotkey::Unicode(entry, Arc::clone(&inputs)));
		
		if !mods_to_release.is_none() {
			let should_mask = Self::should_mask(mods_to_release);
			let size = mods_to_release.count_ones() + (should_mask as u32 * 2);
			KeySender::with_capacity(size as usize).mods_up_masked(mods_to_release, should_mask).send();
		}
		
		unsafe { SendInput(&inputs, size_of::<INPUT>() as i32); }
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
			KeySender::with_capacity(size as usize).mods_down_masked(mods_to_restore, should_mask).send();
		}
		
		true
	}
	
	fn kb_unicode_repeat(
		entry: KeyMods, inputs: Arc<Vec<INPUT>>, key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool
	{
		if key == entry.key {
			unsafe { SendInput(&inputs, size_of::<INPUT>() as i32); }
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
			KeySender::with_capacity(size as usize).mods_down_masked(mods_to_restore, should_mask).send();
		}
		
		Self::map_hotkey(f, KeyMods::new(h.v_mods, key), h)
	}
	
	fn kb_action(entry: KeyMods, action: fn(KeyEvent), h: &mut MutexGuard<'_, Handler>) -> bool {
		let (event, notf) = KeyEvent::new();
		h.curr_h = Some(CurrHotkey::Action(entry, notf));
		
		thread::spawn(move || action(event)); // TODO: h.curr_h = None here?
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
	
	fn map_hotkey(f: fn() -> Hotkey, entry: KeyMods, h: &mut MutexGuard<'_, Handler>) -> bool {
		match f() {
			Hotkey::Default => Self::kb_default(entry, h),
			Hotkey::Suppress => Self::kb_suppress(entry.key, h),
			Hotkey::Remap(mods, key) => Self::kb_remap(entry, KeyMods::new(mods, key), h),
			Hotkey::Unicode(str) => Self::kb_unicode(entry, str, h),
			Hotkey::Action(action) => Self::kb_action(entry, action, h)
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
		
		let _keybd = unsafe {
			Owned::new(SetWindowsHookExW(WH_KEYBOARD_LL, Some(Self::ll_keybd_proc), None, 0).unwrap())
		};
		
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