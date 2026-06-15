use std::{ffi::c_void, ptr, sync::{Arc, Mutex, mpsc}, thread::{self, JoinHandle}};
use std::collections::{HashMap, hash_map::Entry};
use windows::core::{Error, w};
use windows::Win32::{
	Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
	System::{Threading::GetCurrentThreadId, LibraryLoader::GetModuleHandleW},
	UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PostThreadMessageW,
		RegisterClassW, TranslateMessage, WM_QUIT, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP, CREATESTRUCTW, GWLP_USERDATA,
		GetWindowLongPtrW, SetWindowLongPtrW, WM_NCCREATE}};

pub type MsgHandler = fn(HWND, u32, WPARAM, LPARAM) -> isize;

pub struct State {
	handlers: Mutex<HashMap<u32, Vec<MsgHandler>>>,
}

impl State {
	pub fn new(handlers: Mutex<HashMap<u32, Vec<MsgHandler>>>) -> Self {
		Self { handlers }
	}
}

pub struct MainWindow {
	pub hwnd: HWND,
	thread_id: u32,
	jh: JoinHandle<()>,
	state: Arc<State>,
}

impl MainWindow {
	pub fn new() -> Self {
		let state = Arc::new(State::new(Mutex::new(HashMap::new())));
		let state2 = Arc::clone(&state);
		
		let (tx, rx) = mpsc::channel::<(u32, usize)>();
		let jh = thread::spawn(|| win_mq(tx, state2));
		let (thread_id, hwnd) = rx.recv().unwrap();
		
		Self { jh, thread_id, hwnd: HWND(hwnd as *mut c_void), state }
	}
	
	pub fn on_message(&mut self, msg: u32, f: MsgHandler) {
		let mut map = self.state.handlers.lock().unwrap();
		
		match map.entry(msg) {
			Entry::Occupied(mut occupied) => occupied.get_mut().push(f),
			Entry::Vacant(vacant) => _ = vacant.insert(vec![f]),
		}
	}
	
	pub fn exit(self) {
		unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)).unwrap() };
		self.jh.join().unwrap();
	}
}

fn win_mq(tx: mpsc::Sender<(u32, usize)>, state: Arc<State>) {
	let h_inst = HINSTANCE(unsafe { GetModuleHandleW(None).unwrap().0 });
	let class_name = w!("i44_win");
	
	let mut wc = WNDCLASSW::default();
	wc.lpfnWndProc = Some(win_proc);
	wc.hInstance = h_inst;
	wc.lpszClassName = class_name;
	
	if unsafe { RegisterClassW(&wc) } == 0 {
		panic!("failed to register class: {}", Error::from_thread().message());
	}
	
	let r_state = Arc::into_raw(state);
	
	let hwnd = unsafe { CreateWindowExW(
		WS_EX_TOOLWINDOW,
		class_name,
		None,
		WS_POPUP,
		0, 0, 0, 0,
		None, None,
		Some(h_inst),
		Some(r_state as *const c_void)) }.expect("failed to create window");
	
	tx.send((unsafe { GetCurrentThreadId() }, hwnd.0 as usize)).unwrap();
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
	
	_ = unsafe { Arc::from_raw(r_state) };
}

unsafe extern "system" fn win_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	println!("msg: 0x{msg:X}"); // TODO: remove
	
	if msg == WM_NCCREATE {
		let cs = unsafe { ptr::read(lparam.0 as *const CREATESTRUCTW) };
		unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize) };
		return LRESULT(1);
	};
	
	let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State };
	if ptr.is_null() {
		return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
	}
	
	let state = unsafe { &*ptr };
	let map = state.handlers.lock().unwrap();
	
	if let Some(handlers) = map.get(&msg) {
		for h in handlers {
			let ret = h(hwnd, msg, wparam, lparam);
			if ret != 0 {
				return LRESULT(ret);
			}
		}
	}
	
	drop(map);
	unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}