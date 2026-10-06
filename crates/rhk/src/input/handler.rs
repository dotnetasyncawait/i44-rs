use super::{
	hotkey::{Hotkey, HotkeyHandler},
	hotstr::{Hotstr, HotstrTrie},
	mods::Mods, InputKey,
	keys::{Key, KeyType, Led},
	input_builder::InputBuilder,
	extensions::{InputExt, VecDequeExt, FullMode},
	constants::{CALL_NEXT, CALL_NEXT_END, CACHED_EVENT}, key_event::{KeyEvent, KeyEventNotifier}};
use crate::{common::error::{Error, OK}, misc::threadpool};
use std::{collections::{HashMap, hash_map::Entry, VecDeque}, ptr, thread::{self, JoinHandle}, fmt, borrow::Cow};
use std::sync::{mpsc::{self, SyncSender, TrySendError}, Arc, OnceLock, Mutex, MutexGuard, atomic::{AtomicBool, Ordering}};
use windows::core::Owned;
use windows::Win32::{Foundation::{LPARAM, LRESULT, WPARAM}, System::Threading::GetCurrentThreadId};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::{
	Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VK_TO_VSC_EX, VK_BROWSER_BACK, VK_LAUNCH_APP2, INPUT, SendInput,
		GetKeyState, VK_CAPITAL, VK_NUMLOCK, VK_SCROLL},
	WindowsAndMessaging::{WH_KEYBOARD_LL, WH_MOUSE_LL, LLKHF_UP, LLKHF_EXTENDED, LLKHF_INJECTED, MSG, LLMHF_INJECTED,
		MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
		WM_MOUSEHWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_XBUTTONDOWN, WM_XBUTTONUP, XBUTTON1, XBUTTON2, KBDLLHOOKSTRUCT,
		SetWindowsHookExW,GetMessageW, TranslateMessage, DispatchMessageW, CallNextHookEx}};

#[derive(Debug)]
pub struct Handler {
	hotkeys: HashMap<KeyMods, HotkeyHandler>,
	suppressed: HashMap<Key, bool>, // bool: once
	curr_h: Option<CurrHotkey>,
	v_mods: Mods,
	led: Led,
	send_count: u8,
	sender: mpsc::Sender<InputMsg>,
	last_mod: Mods,
	last_h_mods: Mods,
	hotstr_buf: VecDeque<u8>,
	hotstrs: HotstrTrie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct KeyMods { mods: Mods, key: Key }

impl KeyMods {
	fn new(mods: Mods, key: Key) -> Self { Self { mods, key } }
}

impl fmt::Display for KeyMods {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} & {}", self.mods, self.key)
	}
}

enum CurrHotkey {
	Default(KeyMods),
	Remap(KeyMods, KeyMods),
	Unicode(KeyMods, Arc<Vec<INPUT>>),
	Action(KeyMods, KeyEventNotifier),
	ActionRepeat(KeyMods, mpsc::SyncSender<()>),
}

enum InputMsg {
	Single(INPUT),
	Many(Vec<INPUT>),
	Shared(Arc<Vec<INPUT>>),
}

static HANDLER: OnceLock<Mutex<Handler>> = OnceLock::new();
static SUSPENDED: AtomicBool = AtomicBool::new(false);

const PROCESS: LRESULT = LRESULT(0);
const BLOCK: LRESULT = LRESULT(1);

pub fn is_suspended() -> bool {
	SUSPENDED.load(Ordering::Relaxed)
}

pub fn suspend(state: bool) {
	SUSPENDED.store(state, Ordering::Relaxed);
}

pub fn suspend_tgl() -> bool {
	!SUSPENDED.fetch_not(Ordering::Relaxed)
}

impl Handler {
	pub fn new() -> Self {
		let (placeholder, _) = mpsc::channel();
		
		Self {
			hotkeys: HashMap::new(),
			suppressed: HashMap::new(),
			curr_h: None,
			v_mods: Mods::NONE,
			led: Led(0),
			send_count: 0,
			sender: placeholder,
			last_mod: Mods::NONE,
			last_h_mods: Mods::NONE,
			hotstr_buf: VecDeque::with_capacity(32),
			hotstrs: HotstrTrie::new(),
		}
	}
	
	pub fn hotkey(&mut self, mods: Mods, key: Key, f: fn() -> Result<Hotkey, Error>, exempt: bool) {
		match self.hotkeys.entry(KeyMods::new(mods, key)) {
			Entry::Occupied(o) => panic!("hotkey {} already exists", o.key()),
			Entry::Vacant(v) => v.insert_entry(HotkeyHandler::new(f, exempt)),
		};
	}
	
	pub fn hotstr(&mut self, entry: &str, f: fn() -> Result<Hotstr, Error>, exempt: bool) {
		self.hotstrs.add(entry, f, exempt);
	}
	
	pub fn start(mut self) -> (JoinHandle<()>, u32) {
		let (tx, rx) = mpsc::channel::<InputMsg>();
		let _ = thread::spawn(|| {
			const INPUT_SIZE: i32 = size_of::<INPUT>() as _;
			for msg in rx {
				match msg {
					InputMsg::Single(input) => unsafe { SendInput(&[input], INPUT_SIZE); },
					InputMsg::Many(inputs) => unsafe { SendInput(&inputs, INPUT_SIZE); },
					InputMsg::Shared(inputs) => unsafe { SendInput(&inputs, INPUT_SIZE); },
				}
			}
		});
		self.sender = tx;
		
		let (tx, rx) = mpsc::sync_channel::<usize>(0);
		let jh = thread::spawn(|| mq_handler(tx));
		
		let msg = rx.recv().unwrap();
		let led_state = msg as u8 & 0b111;
		let thread_id = (msg >> 32) as u32;
		
		self.led.0 = led_state;
		HANDLER.set(Mutex::new(self)).expect("handler should not be set");
		
		// mq_handler is waiting for us to disconnect so it can start processing input events.
		// Close the receiver only after full state-initialization.
		drop(rx); 
		
		(jh, thread_id)
	}
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
	
	// IMPORTANT: calling the next hook (as mentioned above) may re-enter the hook (if there's an event
	// in the queue), which can cause the order of processing keys to be broken. When this happens, the
	// target window receives the re-entered event before the currently processing one (which is waiting
	// for the CallNextHookEx to return).
	// For example, decrementing `send_count` on `CALL_NEXT_END` before calling the next hook is a violation,
	// because the interrupting keys will be sent before we complete our sequence (QMK shifted-symbols case).
	
	let s = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
	
	match s.dwExtraInfo {
		CALL_NEXT => return PROCESS,
		CALL_NEXT_END => {
			lock_handler().send_count -= 1;
			return PROCESS;
		},
		_ => {}
	}
	
	let Some((key, pressed)) = kb_get_key(&s) else {
		return PROCESS; // injected key
	};
	
	let h = lock_handler();
	if h.send_count != 0 {
		if !pressed {
			h.sender.send(InputMsg::Single(INPUT::keybd_up(key, CACHED_EVENT))).unwrap();
		}
		return BLOCK;
	}
	
	if handle_kb(key, pressed, h) {
		BLOCK
	} else {
		PROCESS
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
			lock_handler().send_count -= 1;
			return call_next(code, wparam, lparam);
		},
		_ => {}
	}
	
	if s.flags & LLMHF_INJECTED != 0 && s.dwExtraInfo != CACHED_EVENT {
		return BLOCK;
	}
	
	let (key, pressed) = ms_get_key(wparam, s.mouseData);
	
	let h = lock_handler();
	if h.send_count != 0 {
		if !pressed {
			h.sender.send(InputMsg::Single(INPUT::mouse_up(key, CACHED_EVENT))).unwrap();
		}
		return BLOCK;
	}
	
	if handle_ms(key, pressed, h) {
		BLOCK
	} else {
		call_next(code, wparam, lparam)
	}
}

fn handle_kb(key: Key, pressed: bool, mut h: MutexGuard<'_, Handler>) -> bool {
	let mod_bit = get_mod(key);
	
	if pressed {
		if kb_key_down(key, mod_bit, &mut h) {
			true
		} else if mod_bit.is_none() {
			if handle_hotstr(key, &mut h) {
				true
			} else {
				h.led.update_state(key);
				false
			}
		} else {
			h.last_mod = mod_bit;
			h.v_mods |= mod_bit;
			false
		}
	} else {
		if kb_key_up(key, mod_bit, &mut h) {
			return true;
		}
		if mod_bit.is_none() {
			return false;
		}
		
		let mut masked = false;
		if h.last_mod == mod_bit && h.last_h_mods.has_any(mod_bit) && should_mask_last(mod_bit, h.v_mods) {
			let ib = InputBuilder::with_capacity(3)
				.key_down(Key::LCTRL)
				.key_up(key)
				.key_up(Key::LCTRL);
			
			h.send_count += 1;
			h.sender.send(InputMsg::Many(ib.build())).unwrap();
			masked = true;
		}
		
		h.v_mods &= !mod_bit;
		h.last_h_mods &= !mod_bit;
		h.last_mod = Mods::NONE;
		masked
	}
}

fn handle_ms(key: Key, pressed: bool, mut h: MutexGuard<'_, Handler>) -> bool {
	if pressed {
		if key.is_mouse_wheel() {
			ms_wheel(key, &mut h)
		} else {
			kb_key_down(key, Mods::NONE, &mut h)
		}
	} else {
		kb_key_up(key, Mods::NONE, &mut h)
	}
}

fn ms_wheel(key: Key, h: &mut Handler) -> bool {
	if let Some(_) = &h.curr_h {
		return false; // TODO what?
	}
	
	let entry = KeyMods::new(h.v_mods, key);
	
	let Some(hh) = get_hotkey(entry, h) else {
		return false;
	};
	
	let hotkey = match (hh.func)() {
		Ok(hotkey) => hotkey,
		Err(err) => {
			display_err(entry, err);
			return true;
		}
	};
	
	h.last_h_mods = entry.mods;
	
	match hotkey {
		Hotkey::Default => false,
		Hotkey::Suppress | Hotkey::SuppressOnce => true,
		Hotkey::Remap(r_mods, r_key) => {
			let remap_mod_bit = get_mod(r_key);
			let remap = KeyMods::new(r_mods, r_key);
			
			let mut entry_mods = entry.mods & !(r_mods | remap_mod_bit);
			let mut remap_mods = r_mods & !entry.mods;
			let mut entry_mods_2 = entry_mods;
			let mut remap_mods_2 = remap_mods;
			
			let (mask, pre_mask, post_mask) = mask_remap_start(&mut entry_mods, &mut remap_mods, entry);
			let (mask_2, pre_mask_2, post_mask_2) = mask_remap_end(&mut remap_mods_2, &mut entry_mods_2, remap);
			let is_wheel = r_key.is_mouse_wheel();
			
			let size = pre_mask as u32
				+ (entry_mods | remap_mods).count_ones()
				+ post_mask as u32
				+ 1
				+ !is_wheel as u32
				+ pre_mask_2 as u32
				+ (remap_mods_2 | entry_mods_2).count_ones()
				+ post_mask_2 as u32;
			
			let ib = InputBuilder::with_capacity(size as _)
				.key_down_if(mask, pre_mask)
				.mods_up(entry_mods)
				.mods_down(remap_mods)
				.key_up_if(mask, post_mask)
				.key_down(r_key)
				.key_up_if(r_key, !is_wheel)
				.key_down_if(mask_2, pre_mask_2)
				.mods_up(remap_mods_2)
				.mods_down(entry_mods_2)
				.key_up_if(mask_2, post_mask_2);
			
			h.last_mod = if !post_mask_2 { get_last_mod(entry_mods_2) } else { Mods::NONE };
			h.send_count += 1;
			h.sender.send(InputMsg::Many(ib.build())).unwrap();
			true
		},
		Hotkey::Unicode(str) => {
			let encoded: Vec<u16> = str.encode_utf16().collect();
			let entry_mods = entry.mods;
			
			let should_mask = h.last_mod.is_any() && matches!(project_mods(entry_mods), Mods::LA_RA | Mods::LW_RW);
			let size = (entry_mods.count_ones() + encoded.len() as u32) * 2 + (should_mask as u32 * 2);
			
			if size != 0 {
				let inputs = InputBuilder::with_capacity(size as _)
					.mods_up_masked(entry_mods, should_mask)
					.add_unicode(encoded)
					.mods_down(entry_mods)
					.build();
				
				h.last_mod = get_last_mod(entry_mods);
				h.send_count += 1;
				h.sender.send(InputMsg::Many(inputs)).unwrap();
			}
			true
		},
		Hotkey::Action(action) => {
			let (notf, event) = KeyEvent::new();
			// Mouse wheel has only `down` state, so the event will always report 'up'.
			notf.notify();
			// Since there's no 'up' event for wheel-keys, we don't store the currently performing hotkey.
			// Hence, it will allow to run a new action before the previous one has completed.
			// It could be fixed by notifying back from the runner thread (once the action is run to completion),
			// but it would require adding a new field to the Handler/CurrHotkey::Action, specifically for this case.
			// Since actions on wheel keys are rare (not any for now), it should not be a problem.
			queue_job(move || {
				if let Err(err) = action(event) {
					display_err(entry, err);
				}
				OK
			});
			true
		},
		Hotkey::ActionRepeat(action) => {
			queue_job(move || {
				if let Err(err) = action() {
					display_err(entry, err);
				}
				OK
			});
			true
		},
	}
}

fn kb_key_down(key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	if let Entry::Occupied(entry) = h.suppressed.entry(key) {
		if *entry.get() { // suppressed once
			entry.remove();
		} else {
			return true;
		}
	}
	
	if let Some(curr_h) = &h.curr_h {
		return match curr_h {
			CurrHotkey::Default(entry) => kb_default_repeat(*entry, key, mod_bit),
			CurrHotkey::Remap(entry, remap) => kb_remap_repeat(*entry, *remap, key, mod_bit, h),
			CurrHotkey::Unicode(entry, inputs) => kb_unicode_repeat(*entry, Arc::clone(&inputs), key, mod_bit, h),
			CurrHotkey::Action(entry, _) => kb_action_repeat(*entry, key, mod_bit),
			CurrHotkey::ActionRepeat(entry, sender) => kb_action_r_repeat(*entry, &sender, key, mod_bit),
		};
	}
	
	let entry = KeyMods::new(h.v_mods, key);
	
	if let Some(hh) = get_hotkey(entry, h) {
		map_hotkey(hh.func, entry, h)
	} else {
		false
	}
}

fn kb_key_up(key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	if h.suppressed.remove(&key).is_some() {
		return true;
	}
	
	match &h.curr_h {
		Some(curr_h) => match curr_h {
			CurrHotkey::Default(entry) => kb_default_up(*entry, key, mod_bit, h),
			CurrHotkey::Remap(entry, remap) => kb_remap_up(*entry, *remap, key, mod_bit, h),
			CurrHotkey::Unicode(entry, _) => kb_unicode_up(*entry, key, mod_bit, h),
			CurrHotkey::Action(entry, notf) => kb_action_up(*entry, notf.clone(), key, mod_bit, h),
			CurrHotkey::ActionRepeat(entry, _) => kb_action_r_up(*entry, key, mod_bit, h),
		},
		None => false
	}
}

fn kb_default(entry: KeyMods, h: &mut Handler) -> bool {
	h.curr_h = Some(CurrHotkey::Default(entry));
	false
}

fn kb_default_repeat(entry: KeyMods, key: Key, mod_bit: Mods) -> bool {
	if entry.key == key {
		false
	} else {
		// TODO: map hotkey if there's a match
		entry.mods.has_any(mod_bit)
	}
}

fn kb_default_up(entry: KeyMods, key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	if entry.key == key || entry.mods.has_any(mod_bit) {
		h.curr_h = None;
	}
	false
}

fn kb_suppress(entry: KeyMods, h: &mut Handler, once: bool) -> bool {
	h.suppressed.insert(entry.key, once);
	h.last_h_mods = entry.mods;
	true
}

fn kb_remap(entry: KeyMods, remap: KeyMods, h: &mut Handler) -> bool {
	let remap_mod_bit = get_mod(remap.key);
	let mut mods_to_release = entry.mods & !(remap.mods | remap_mod_bit);
	let mut mods_to_press = remap.mods & !entry.mods;
	
	let (mask, pre_mask, post_mask) = mask_remap_start(&mut mods_to_release, &mut mods_to_press, entry);
	let size = pre_mask as u32 + (mods_to_release | mods_to_press).count_ones() + post_mask as u32 + 1;
	
	let inputs = InputBuilder::with_capacity(size as _)
		.key_down_if(mask, pre_mask)
		.mods_up(mods_to_release)
		.mods_down(mods_to_press)
		.key_up_if(mask, post_mask)
		.key_down(remap.key)
		.build();
	
	// h.last_mod = if !post_mask && remap.key.is_mouse_key() { get_last_mod(mods_to_press) } else { Mods::NONE };
	h.v_mods = remap.mods | remap_mod_bit;
	h.curr_h = Some(CurrHotkey::Remap(entry, remap));
	h.send_count += 1;
	h.sender.send(InputMsg::Many(inputs)).unwrap();
	
	true
}

fn kb_remap_up(entry: KeyMods, remap: KeyMods, key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	let key_to_release = remap.key;
	
	let (mut mods_to_release, mut mods_to_restore, last_h_mods) = if key == entry.key {
		( remap.mods & !entry.mods,
			entry.mods & !remap.mods,
			entry.mods)
	} else if entry.mods.has_any(mod_bit) {
		h.suppressed.insert(entry.key, false);
		( remap.mods & (!entry.mods | mod_bit),
			entry.mods & !(remap.mods | mod_bit),
			entry.mods & !mod_bit)
	} else {
		return false;
	};
	
	h.curr_h = None;
	h.v_mods = h.v_mods & !(mods_to_release | get_mod(key_to_release)) | mods_to_restore;
	
	let is_wheel = key_to_release.is_mouse_wheel();
	let (mask, pre_mask, post_mask) = mask_remap_end(&mut mods_to_release, &mut mods_to_restore, remap);
	
	let size = !is_wheel as u32
		+ pre_mask as u32
		+ (mods_to_release | mods_to_restore).count_ones()
		+ post_mask as u32;
	
	if size != 0 {
		let inputs = InputBuilder::with_capacity(size as _)
			.key_up_if(key_to_release, !is_wheel)
			.key_down_if(mask, pre_mask)
			.mods_up(mods_to_release)
			.mods_down(mods_to_restore)
			.key_up_if(mask, post_mask)
			.build();
		
		h.last_h_mods = last_h_mods;
		h.last_mod = if !post_mask { get_last_mod(mods_to_restore) } else { Mods::NONE };
		h.send_count += 1;
		h.sender.send(InputMsg::Many(inputs)).unwrap();
	}
	
	true
}

fn kb_remap_repeat(entry: KeyMods, remap: KeyMods, key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	if key == entry.key {
		let key_to_repeat = remap.key;
		if !key_to_repeat.is_mouse_button() { // we don't repeat mouse buttons
			
			// We could check if key == key_to_repeat and simply return false here (instead of simulating input), 
			// but that would make it impossible for the topmost caller to distinguish whether the key is actually
			// handled, leading to erroneously calling a hotstring handler.
			// Changing the signature to Option<bool> would also require all the subsequent/neighboring handlers
			// to be changed as well. Considering that this case is rare (mods_1+k -> mods_2+k), it's probably not
			// worth it.
			
			let input = if key_to_repeat.is_mouse_wheel() {
				INPUT::mouse_down(key_to_repeat, CALL_NEXT_END)
			} else {
				INPUT::keybd_down(key_to_repeat, CALL_NEXT_END)
			};
			h.send_count += 1;
			h.sender.send(InputMsg::Single(input)).unwrap();
		}
		return true;
	}
	
	if entry.mods.has_any(mod_bit) { // suppress repeated entry mods (Qmk KeyOverrides issue)
		return true;
	}
	
	if get_mod(remap.key).is_any() { // key-to-mod remap
		return false;
	}
	
	let ph_entry = KeyMods::new(h.v_mods & !remap.mods | entry.mods, key);
	
	let Some(hh) = get_hotkey(ph_entry, h) else {
		return false;
	};
	
	// interrupt
	
	h.curr_h = None;
	h.v_mods = ph_entry.mods;
	h.suppressed.insert(entry.key, false);
	
	let key_to_release = remap.key;
	let mut mods_to_release = remap.mods & !entry.mods;
	let mut mods_to_restore = entry.mods & !remap.mods;
	
	let is_wheel = key_to_release.is_mouse_wheel();
	let (mask, pre_mask, post_mask) = mask_remap_end(&mut mods_to_release, &mut mods_to_restore, remap);
	
	let size = !is_wheel as u32
		+ pre_mask as u32
		+ (mods_to_release | mods_to_restore).count_ones()
		+ post_mask as u32;
	
	if size != 0 {
		let inputs = InputBuilder::with_capacity(size as _)
			.key_up_if(key_to_release, !is_wheel)
			.key_down_if(mask, pre_mask)
			.mods_up(mods_to_release)
			.mods_down(mods_to_restore)
			.key_up_if(mask, post_mask)
			.build();
		
		h.send_count += 1;
		h.sender.send(InputMsg::Many(inputs)).unwrap();
	}
	
	map_hotkey(hh.func, ph_entry, h)
}

fn kb_unicode(entry: KeyMods, str: &'static str, h: &mut Handler) -> bool {
	if str.len() == 0 {
		return true;
	}
	
	let inputs = Arc::new(InputBuilder::unicode(str));
	let mods_to_release = entry.mods;
	
	h.v_mods = Mods::NONE;
	h.curr_h = Some(CurrHotkey::Unicode(entry, Arc::clone(&inputs)));
	
	if mods_to_release.is_any() {
		let should_mask = h.last_mod.is_any() && matches!(project_mods(mods_to_release), Mods::LA_RA | Mods::LW_RW);
		let size = mods_to_release.count_ones() + (should_mask as u32 * 2);
		
		let keys = InputBuilder::with_capacity(size as _)
			.mods_up_masked(mods_to_release, should_mask)
			.build();
		
		h.send_count += 1;
		h.sender.send(InputMsg::Many(keys)).unwrap();
	}
	
	h.send_count += 1;
	h.sender.send(InputMsg::Shared(inputs)).unwrap();
	true
}

fn kb_unicode_up(entry: KeyMods, key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	let mods_to_restore;
	
	if key == entry.key {
		mods_to_restore = entry.mods;
	} else if entry.mods.has_any(mod_bit) {
		mods_to_restore = entry.mods & !mod_bit;
		h.suppressed.insert(entry.key, false);
	} else {
		return false;
	}
	
	h.curr_h = None;
	h.v_mods |= mods_to_restore;
	
	if !mods_to_restore.is_none() {
		let inputs = InputBuilder::with_capacity(mods_to_restore.count_ones() as _)
			.mods_down(mods_to_restore)
			.build();
		
		h.last_h_mods = mods_to_restore;
		h.last_mod = get_last_mod(mods_to_restore);
		h.send_count += 1;
		h.sender.send(InputMsg::Many(inputs)).unwrap();
	}
	
	true
}

fn kb_unicode_repeat(entry: KeyMods, inputs: Arc<Vec<INPUT>>, key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	if key == entry.key {
		h.send_count += 1;
		h.sender.send(InputMsg::Shared(inputs)).unwrap();
		return true;
	}
	
	if entry.mods.has_any(mod_bit) {
		return true;
	}
	
	let ph_entry = KeyMods::new(h.v_mods | entry.mods, key);
	
	let Some(hh) = get_hotkey(ph_entry, h) else {
		return false;
	};
	
	// interrupt
	
	let mods_to_restore = entry.mods;
	
	h.curr_h = None;
	h.v_mods |= mods_to_restore;
	h.suppressed.insert(entry.key, false);
	
	if !mods_to_restore.is_none() {
		let inputs = InputBuilder::with_capacity(mods_to_restore.count_ones() as _)
			.mods_down(mods_to_restore)
			.build();
		
		h.send_count += 1;
		h.sender.send(InputMsg::Many(inputs)).unwrap();
	}
	
	map_hotkey(hh.func, ph_entry, h)
}

fn kb_action(entry: KeyMods, action: fn(KeyEvent) -> Result<(), Error>, h: &mut Handler) -> bool {
	let (notf, event) = KeyEvent::new();
	h.curr_h = Some(CurrHotkey::Action(entry, notf));
	
	queue_job(move || {
		if let Err(err) = action(event) {
			display_err(entry, err);
		}
		OK
	});
	
	true
}

fn kb_action_up(entry: KeyMods, notf: KeyEventNotifier, key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	kb_action_up_common(|| notf.notify(), entry, key, mod_bit, h)
}

fn kb_action_repeat(entry: KeyMods, key: Key, mod_bit: Mods) -> bool {
	key == entry.key || entry.mods.has_any(mod_bit)
}

fn kb_action_r(entry: KeyMods, action: fn() -> Result<(), Error>, h: &mut Handler) -> bool {
	let (tx, rx) = mpsc::sync_channel(0);
	h.curr_h = Some(CurrHotkey::ActionRepeat(entry, tx));
	
	queue_job(move || {
		loop {
			if let Err(err) = action() {
				display_err(entry, err);
				break;
			}
			if rx.recv().is_err() {
				break;
			}
		}
		OK
	});
	
	true
}

fn kb_action_r_up(entry: KeyMods, key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	kb_action_up_common(|| {}, entry, key, mod_bit, h)
}

fn kb_action_r_repeat(entry: KeyMods, sender: &SyncSender<()>, key: Key, mod_bit: Mods) -> bool {
	if key == entry.key {
		if let Err(err) = sender.try_send(()) && err == TrySendError::Disconnected(()) {
			panic!("Receiver 'ActionRepeat' disconnected");
		}
		true
	} else {
		entry.mods.has_any(mod_bit)
	}
}

fn kb_action_up_common(on_key_up: impl FnOnce(), entry: KeyMods, key: Key, mod_bit: Mods, h: &mut Handler) -> bool {
	if key == entry.key {
		on_key_up();
		h.curr_h = None;
		h.last_h_mods = entry.mods & h.v_mods;
		true
	} else if entry.mods.has_any(mod_bit) && mod_bit == h.last_mod && should_mask_last(mod_bit, h.v_mods) {
		let ib = InputBuilder::with_capacity(3)
			.key_down(Key::LCTRL)
			.key_up(key)
			.key_up(Key::LCTRL);
		
		h.v_mods &= !mod_bit;
		h.last_h_mods &= !mod_bit;
		h.last_mod = Mods::NONE;
		h.send_count += 1;
		h.sender.send(InputMsg::Many(ib.build())).unwrap();
		true
	} else {
		false
	}
}

fn handle_hotstr(key: Key, h: &mut Handler) -> bool {
	if key == Key::SPACE {
		if h.hotstr_buf.is_empty() || h.v_mods.has_any(Mods::LCAW | Mods::RCAW) {
			return false;
		}
		
		let Some((hh, entry)) = h.hotstrs.find(&h.hotstr_buf) else {
			h.hotstr_buf.push_back_on_full(b' ', FullMode::DropOldest);
			return false;
		};
		
		h.hotstr_buf.clear();
		let entry_len = entry.len();
		
		let hotstr = match hh() {
			Ok(res) => res,
			Err(err) => {
				hotstr_erase(entry_len, h);
				display_err(format!("{entry:?}"), err);
				return true;
			}
		};
		
		match hotstr {
			Hotstr::Default => return false,
			Hotstr::Erase => hotstr_erase(entry_len, h),
			Hotstr::Input { r, clear } => {
				let mut ib = InputBuilder::with_capacity((clear as usize * (2*entry_len)) + r.1);
				
				if clear {
					ib = ib.key_tap(Key::BS, entry_len as u16);
				}
				
				for key in r.0.iter().copied() {
					match key {
						InputKey::Unicode(high, low) => ib = ib.add_char(high, low),
						InputKey::Down(sc) => ib = ib.key_down(Key(sc)),
						InputKey::Up(sc) => ib = ib.key_up(Key(sc)),
						InputKey::Tap(sc, times) => ib = ib.key_tap(Key(sc), times),
					}
				}
				
				h.send_count += 1;
				h.sender.send(InputMsg::Many(ib.build())).unwrap();
			}
			Hotstr::Clipb { r, clear, restore } => {
				if let Err(err) = hotstr_clipb(r, clear, restore, entry_len, h) {
					display_err(format!("Hotstr::Clipb({entry:?})"), err);
				}
			}
			Hotstr::Action(action) => {
				hotstr_erase(entry_len, h);
				queue_job(move || {
					if let Err(err) = action() {
						display_err(format!("Hotstr::Action({entry:?})"), err);
					}
					OK
				});
			},
		}
		
		true
	} else {
		let Some(info) = key.info() else {
			return false;
		};
		
		match info.0 {
			KeyType::Letter => {
				if !h.v_mods.has_any(Mods::LCAW | Mods::RCAW) {
					let alt = h.v_mods.has_any(Mods::LS_RS) ^ h.led.is_capslock_on();
					let l = if alt { info.2 } else { info.1 };
					h.hotstr_buf.push_back_on_full(l, FullMode::DropOldest);
				}
			}
			KeyType::Symbol | KeyType::Number => {
				if !h.v_mods.has_any(Mods::LCAW | Mods::RCAW) {
					let l = if h.v_mods.has_any(Mods::LS_RS) { info.2 } else { info.1 };
					h.hotstr_buf.push_back_on_full(l, FullMode::DropOldest);
				}
			}
			KeyType::Navigation => {
				if h.hotstr_buf.is_empty() {
					return false;
				}
				if key == Key::BS {
					if h.v_mods.is_none() {
						h.hotstr_buf.pop_back();
					} else if h.v_mods.has_any(Mods::LC_RC) {
						if !h.v_mods.has_any(Mods::LSAW | Mods::RSAW) {
							h.hotstr_buf.clear();
						}
					} else if h.v_mods.has_any(Mods::LS_RS) {
						if !h.v_mods.has_any(Mods::LAW | Mods::RAW) {
							h.hotstr_buf.pop_back();
						}
					}
				} else if key != Key::INSERT && key != Key::DEL {
					h.hotstr_buf.clear();
				}
			}
		}
		
		false
	}
}

fn hotstr_erase(entry_len: usize, h: &mut Handler) {
	let inputs = InputBuilder::with_capacity(entry_len*2)
		.key_tap(Key::BS, entry_len as u16)
		.build();
	
	h.send_count += 1;
	h.sender.send(InputMsg::Many(inputs)).unwrap();
}

fn hotstr_clipb(r: Cow<'_, str>, clear: bool, restore: bool, entry_len: usize, h: &mut Handler) -> Result<(), Error> {
	use crate::{app::clipboard, misc::timer};
	
	let prev = if restore {
		Some(clipboard::get_raw()?)
	} else {
		None
	};
	
	let mut ib = InputBuilder::with_capacity(4 + (clear as usize * (entry_len*2)));
	if clear {
		ib = ib.key_tap(Key::BS, entry_len as u16);
	}
	
	let inputs = ib
		.key_down(Key::LCTRL)
		.key_down(Key::V)
		.key_up(Key::V)
		.key_up(Key::LCTRL)
		.build();
	
	h.send_count += 1;
	h.sender.send(InputMsg::Many(inputs)).unwrap();
	
	clipboard::set_text(r)?;
	
	if let Some(raw) = prev {
		timer::set_once(200, || Ok(clipboard::set_raw(raw)?))?;
	}
	
	Ok(())
}

fn map_hotkey(f: fn() -> Result<Hotkey, Error>, entry: KeyMods, h: &mut Handler) -> bool {
	match f() {
		Ok(hotkey) => match hotkey {
			Hotkey::Default => kb_default(entry, h),
			Hotkey::Suppress => kb_suppress(entry, h, false),
			Hotkey::SuppressOnce => kb_suppress(entry, h, true),
			Hotkey::Remap(mods, key) => kb_remap(entry, KeyMods::new(mods, key), h),
			Hotkey::Unicode(str) => kb_unicode(entry, str, h),
			Hotkey::Action(action) => kb_action(entry, action, h),
			Hotkey::ActionRepeat(action) => kb_action_r(entry, action, h),
		}
		Err(err) => {
			display_err(entry, err);
			true
		}
	}
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

fn get_hotkey(entry: KeyMods, h: &Handler) -> Option<HotkeyHandler> {
	if let Some(hh) = h.hotkeys.get(&entry) && (hh.exempt || !SUSPENDED.load(Ordering::Relaxed)) {
		Some(*hh)
	} else {
		None
	}
}

fn mq_handler(tx: mpsc::SyncSender<usize>) {
	unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).unwrap(); }
	
	let _keybd = unsafe { Owned::new(SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_keybd_proc), None, 0).unwrap()) };
	let _mouse = unsafe { Owned::new(SetWindowsHookExW(WH_MOUSE_LL, Some(ll_mouse_proc), None, 0).unwrap()) };
	
	let thread_id = unsafe { GetCurrentThreadId() };
	let caps = (unsafe { GetKeyState(VK_CAPITAL.0 as i32) } & 1) as u8;
	let num = (unsafe { GetKeyState(VK_NUMLOCK.0 as i32) } & 1) as u8;
	let scroll = (unsafe { GetKeyState(VK_SCROLL.0 as i32) } & 1) as u8;
	
	let msg = ((thread_id as usize) << 32) | ((scroll << 2) | (num << 1) | caps) as usize;
	tx.send(msg).unwrap();
	// Wait until the receiver disconnects, so it has enough time to set the data
	// and finish the initialization.
	tx.send(0).unwrap_err();
	drop(tx);
	
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
	unsafe { CoUninitialize(); }
}

fn get_last_mod(mods: Mods) -> Mods {
	if mods.is_none() {
		return mods;
	}
	
	const BITS: [u8; 16] = [ 0, 1, 2, 2, 4, 4, 4, 4, 8, 8, 8, 8, 8, 8, 8, 8 ];
	
	let r = BITS[(mods.0 >> 4) as usize];
	let l = BITS[(mods.0 & 0xF) as usize];
	
	Mods(if r >= l { r << 4 } else { l })
}

fn get_mod_except(exclude: Mods, mods: Mods) -> Option<KeyMods> {
	let mods = mods & !exclude;
	if mods.is_none() {
		return None;
	}
	
	const MODS: [(Mods, Key, Key); 4] = [
		(Mods::LC, Key::LCTRL,  Key::RCTRL),
		(Mods::LS, Key::LSHIFT, Key::RSHIFT),
		(Mods::LA, Key::LALT,   Key::RALT),
		(Mods::LW, Key::LWIN,   Key::RWIN)
	];
	
	let l = mods.0 & 0xF;
	let r = mods.0 >> 4;
	
	let l = l & l.wrapping_neg(); // get the lowest bit
	let r = r & r.wrapping_neg();
	
	Some(if r == 0 || (l != 0 && l <= r) {
		let (mods, l_key, _) = MODS[l.trailing_zeros() as usize];
		KeyMods::new(mods, l_key)
	} else {
		let (mods, _, r_key) = MODS[r.trailing_zeros() as usize];
		KeyMods::new(Mods(mods.0 << 4), r_key)
	})
}

fn project_mods(mod_bit: Mods) -> Mods {
	Mods(((mod_bit.0 >> 4) | (mod_bit.0 & 0xF)) * 0x11)
}

fn should_mask_last(last_mod: Mods, curr_mods: Mods) -> bool {
	if last_mod.has_any(Mods::LA_RA) {
		!curr_mods.has_any(Mods::LCS_RCS)
	} else if last_mod.has_any(Mods::LW_RW) {
		!curr_mods.has_any(Mods::LCSA_RCSA)
	} else {
		false
	}
}

fn should_mask_projected(mods_up: Mods, curr_mods: Mods) -> bool {
	debug_assert!((mods_up.0 >> 4) == (mods_up.0 & 0xF), "mods must be projected: {mods_up:?}");
	matches!(mods_up, Mods::LA_RA | Mods::LW_RW) && (curr_mods & !mods_up).0 == 0
}

fn mask_remap_start(mods_up: &mut Mods, mods_down: &mut Mods, entry: KeyMods) -> (Key, bool, bool) {
	let mods = project_mods(*mods_up);
	mask_remap(should_mask_projected(mods, entry.mods), mods, mods_up, mods_down)
}

fn mask_remap_end(mods_up: &mut Mods, mods_down: &mut Mods, remap: KeyMods) -> (Key, bool, bool) {
	let mods = project_mods(*mods_up);
	mask_remap(remap.key.is_mouse_key() && should_mask_projected(mods, remap.mods), mods, mods_up, mods_down)
}

fn mask_remap(should_mask: bool, pr_mods: Mods, mods_up: &mut Mods, mods_down: &mut Mods) -> (Key, bool, bool) {
	//    (k|m) -> (k|m)
	// (W, k|m) -> (k|m):    entry needs mask
	// (W, k|m) -> (C, k|m): entry needs mask + remap has it 
	// (W, k|m) -> (A, k|m): entry and remap need mask
	
	//    (k|m) -> (W, k|m): remap needs mask
	// (C, k|m) -> (W, k|m): remap needs mask + entry has it 
	// (A, k|m) -> (W, k|m): entry and remap need mask
	
	if should_mask {
		if let Some(mask) = get_mod_except(pr_mods, *mods_down) {
			*mods_down &= !mask.mods;
			(mask.key, true, false)
		} else {
			(Key::LCTRL, true, true)
		}
	} else {
		// If unconditionally masking 'mods_down' turns out to be a problem – fix here:
		if let Some(mask) = get_mod_except(project_mods(*mods_down), *mods_up) {
			*mods_up &= !mask.mods;
			(mask.key, false, true)
		} else {
			(Key::NONE, false, false)
		}
	}
}

fn display_err(entry: impl fmt::Display, err: Error) {
	// TODO: display with a window
	println!("entry({entry}): {err}");
}

fn queue_job(job: impl FnOnce() -> Result<(), Error> + Send + 'static) {
	threadpool::queue_job(job).expect("failed to queue job")
}

fn call_next(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

impl fmt::Debug for CurrHotkey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Default(entry) => f.debug_tuple("Default").field(entry).finish(),
			Self::Remap(entry, remap) => f.debug_tuple("Remap").field(entry).field(remap).finish(),
			Self::Unicode(entry, _) => f.debug_tuple("Unicode").field(entry).finish(), // TODO
			Self::Action(entry, notf) => f.debug_tuple("Action").field(entry).field(notf).finish(),
			Self::ActionRepeat(entry, sender) => f.debug_tuple("ActionRepeat").field(entry).field(sender).finish(),
		}
	}
}
