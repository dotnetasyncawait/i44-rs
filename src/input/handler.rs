use super::{hotkey::Hotkey, mods::Mods, keys::Key};
use std::{collections::HashMap, ptr, sync::{mpsc, OnceLock, Mutex}, thread::{self, JoinHandle}};
use std::collections::hash_map::Entry;
use windows::core::Owned;
use windows::Win32::{Foundation::{LPARAM, LRESULT, WPARAM}, System::Threading::GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{WH_KEYBOARD_LL, WM_QUIT, MSG, KBDLLHOOKSTRUCT, SetWindowsHookExW,
	GetMessageW, TranslateMessage, DispatchMessageW, CallNextHookEx, PostThreadMessageW};

#[derive(Debug)]
pub struct Handler {
	hotkeys: HashMap<(Mods, Key), fn() -> Hotkey>,
}

#[derive(Debug)]
struct Worker(Option<JoinHandle<()>>, u32);

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
		Self { hotkeys: HashMap::new() }
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
		println!("sc: 0x{:X}", s.scanCode);
		
		unsafe { CallNextHookEx(None, code, wparam, lparam) }
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