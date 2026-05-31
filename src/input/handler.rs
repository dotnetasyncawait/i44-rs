use super::{hotkey::Hotkey, mods::Mods, keys::Key};
use std::{collections::{HashMap, HashSet}, ptr, sync::{mpsc, OnceLock, Mutex, MutexGuard}, thread::{self, JoinHandle}};
use std::collections::hash_map::Entry;
use windows::core::Owned;
use windows::Win32::{Foundation::{LPARAM, LRESULT, WPARAM}, System::Threading::GetCurrentThreadId};
use windows::Win32::UI::{
	Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VK_TO_VSC_EX, VK_BROWSER_BACK, VK_LAUNCH_APP2},
	WindowsAndMessaging::{
		WH_KEYBOARD_LL, WM_QUIT, LLKHF_UP, LLKHF_EXTENDED, LLKHF_INJECTED,
		MSG, KBDLLHOOKSTRUCT,
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

#[derive(Debug)]
enum CurrHotkey {
	Default(KeyMods),
	Remap(KeyMods, KeyMods),
	Unicode(KeyMods, /* Vec<INPUT> */),
	Action(KeyMods, /* KeyEvent */)
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
		
		// TODO: filter 
		
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
		
		if Self::kb_key(Key(sc), pressed) {
			LRESULT(1)
		} else {
			unsafe { CallNextHookEx(None, code, wparam, lparam) }
		}
	}
	
	fn kb_key(key: Key, pressed: bool) -> bool {
		let mut h = HANDLER.get().unwrap().lock().unwrap();
		let mod_bit = Self::get_mod(key);
		
		if pressed {
			if Self::kb_key_down(key, mod_bit, &mut h) {
				return true
			}
			h.v_mods |= mod_bit;
		} else {
			if Self::kb_key_up(key, mod_bit, &mut h) {
				return true
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
				CurrHotkey::Remap(_, _) => todo!(),
				CurrHotkey::Unicode(_) => todo!(),
				CurrHotkey::Action(_) => todo!()
			};
		}
		
		let Some(&f) = h.hotkeys.get(&(h.v_mods, key)) else {
			return false;
		};
		
		let entry = KeyMods::new(h.v_mods, key);
		
		match f() {
			Hotkey::Default => Self::kb_default(entry, h),
			Hotkey::Suppress => Self::kb_suppress(key, h),
			Hotkey::Remap(_, _) => todo!(),
			Hotkey::Unicode(_) => todo!(),
			Hotkey::Action(_) => todo!()
		}
	}
	
	fn kb_key_up(key: Key, mod_bit: Mods, h: &mut MutexGuard<'_, Handler>) -> bool {
		if h.suppressed.remove(&key) {
			return true;
		}
		
		match &h.curr_h {
			Some(curr_h) => match curr_h {
				CurrHotkey::Default(entry) => Self::kb_default_up(*entry, key, mod_bit, h),
				CurrHotkey::Remap(_, _) => todo!(),
				CurrHotkey::Unicode(_) => todo!(),
				CurrHotkey::Action(_) => todo!()
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