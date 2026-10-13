use std::{cell::RefCell, ffi::c_void, sync::{Arc, Mutex, Weak, mpsc}, fmt};
use std::{thread::{self, JoinHandle}, collections::{HashMap, hash_map::Entry}};
use super::tray_icon::{TrayIcon, IconBuilder, IconEvent};
use crate::common::error::{Error};
use windows::core::w;
use windows::Win32::{
	Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
	System::LibraryLoader::GetModuleHandleW,
	UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW,
		TranslateMessage, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP, CREATESTRUCTW, GWLP_USERDATA, GetWindowLongPtrW,
		SetWindowLongPtrW, WM_NCCREATE, WM_USER, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN,
		WM_CLOSE, WM_DESTROY, WM_ENDSESSION, PostMessageW, PostQuitMessage, DestroyWindow}};

pub type OnMsgCallback = fn(HWND, u32, WPARAM, LPARAM) -> Result<Option<isize>, Error>;
pub type OnExitCallback = fn() -> bool;

pub(crate) const WM_ICON_MSG: u32 = WM_USER | 0xFF;

pub struct State {
	on_msg_cbs: Mutex<HashMap<u32, Vec<OnMsgCallback>>>,
	on_exit_cbs: Mutex<Vec<OnExitCallback>>,
	pub(super) icons: Mutex<HashMap<u32, TrayIcon>>,
}

impl State {
	pub fn new() -> Self {
		Self {
			on_msg_cbs: Mutex::new(HashMap::new()),
			on_exit_cbs: Mutex::new(Vec::new()),
			icons: Mutex::new(HashMap::new()),
		}
	}
}

#[derive(Debug)]
pub struct MainWindow {
	pub hwnd: HWND,
	jh: RefCell<Option<JoinHandle<()>>>,
	// The strong reference is owned by the message queue thread.
	// State is assumed to be alive if Self is alive (until Self::exit is called).
	state: Weak<State>,
}

impl MainWindow {
	pub fn new() -> Self {
		let state = Arc::new(State::new());
		let weak_state = Arc::downgrade(&state);
		
		let (tx, rx) = mpsc::channel::<usize>();
		let jh = thread::spawn(|| win_mq(tx, state));
		let hwnd = HWND(rx.recv().unwrap() as _);
		
		Self { jh: RefCell::new(Some(jh)), hwnd, state: weak_state }
	}
	
	pub fn on_message(&self, msg: u32, f: OnMsgCallback) {
		if let Some(state) = self.state.upgrade() {
			let mut map = state.on_msg_cbs.lock().unwrap();
			
			match map.entry(msg) {
				Entry::Occupied(mut occupied) => occupied.get_mut().push(f),
				Entry::Vacant(vacant) => _ = vacant.insert(vec![f]),
			}
		}
	}
	
	pub fn on_exit(&self, f: OnExitCallback) {
		if let Some(state) = self.state.upgrade() {
			state.on_exit_cbs.lock().unwrap().push(f);
		}
	}
	
	pub fn icon_builder(&self) -> IconBuilder {
		IconBuilder::new(self.hwnd, self.state.clone())
	}
	
	pub fn wait(&self) {
		self.jh
			.borrow_mut()
			.take().expect("main window must only be awaited once")
			.join().unwrap();
	}
	
	pub fn exit(&self) {
		_ = unsafe { PostMessageW(Some(self.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
	}
}

unsafe impl Sync for MainWindow {}
unsafe impl Send for MainWindow {}

fn win_mq(tx: mpsc::Sender<usize>, state: Arc<State>) {
	let h_inst = HINSTANCE(unsafe { GetModuleHandleW(None).unwrap().0 });
	let class_name = w!("i44_win");
	
	let mut wc = WNDCLASSW::default();
	wc.lpfnWndProc = Some(win_proc);
	wc.hInstance = h_inst;
	wc.lpszClassName = class_name;
	
	if unsafe { RegisterClassW(&wc) } == 0 {
		panic!("failed to register class: {}", windows_core::Error::from_thread());
	}
	
	let hwnd = unsafe { CreateWindowExW(
		WS_EX_TOOLWINDOW,
		class_name,
		w!("i44 main window"),
		WS_POPUP,
		0, 0, 0, 0,
		None, None,
		Some(h_inst),
		Some(Arc::into_raw(state) as *const c_void)) }.expect("failed to create window");
	
	tx.send(hwnd.0 as usize).unwrap();
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
}

unsafe extern "system" fn win_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	match msg {
		WM_NCCREATE => {
			let cs = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
			unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize); }
			return def_win_proc(hwnd, msg, wparam, lparam);
		},
		WM_ICON_MSG if lparam.0 as u32 == WM_MOUSEMOVE => return LRESULT(0),
		_ => {}
	}
	
	let r_state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State };
	if r_state.is_null() {
		return def_win_proc(hwnd, msg, wparam, lparam);
	}
	
	let state = unsafe { &*r_state };
	
	match msg {
		WM_CLOSE => {
			let exit_cbs = state.on_exit_cbs.lock().unwrap();
			for f in exit_cbs.iter() {
				if f() {
					return LRESULT(0);
				}
			}
			drop(exit_cbs);
			
			let mut state = unsafe { Arc::from_raw(r_state) };
			let mut freed = false;
			
			// There might be tray icon creation/deletion holding an upgraded, strong reference to the state.
			// Might want to add better synchronization primitive instead of busy-wait.
			for _ in 0..100_000 {
				match Arc::try_unwrap(state) {
					Ok(_) => { freed = true; break }
					Err(s) => state = s
				}
				std::hint::spin_loop();
			}
			
			assert!(freed, "failed to free the state");
			unsafe { DestroyWindow(hwnd).unwrap() };
		}
		WM_DESTROY => unsafe { PostQuitMessage(0); },
		WM_ENDSESSION if wparam.0 == 1 => {
			let exit_cbs = state.on_exit_cbs.lock().unwrap();
			for f in exit_cbs.iter() {
				if f() {
					break;
				}
			}
		}
		WM_ICON_MSG => {
			let event = match lparam.0 as u32 {
				WM_LBUTTONDOWN => IconEvent::LClick,
				WM_LBUTTONDBLCLK => IconEvent::DClick,
				WM_RBUTTONDOWN => IconEvent::RClick,
				_ => return LRESULT(0)
			};
			
			let icons = state.icons.lock().unwrap();
			let icon_id = wparam.0 as u32;
			
			if let Some(icon) = icons.get(&icon_id)
				&& let Some(cb) = icon.handler
				&& let Err(err) = cb(icon, event)
			{
				display_err(format!("icon_handler({icon_id}): {err}"));
			}
		}
		_ => {
			let msg_cbs_map = state.on_msg_cbs.lock().unwrap();
			if let Some(msg_cbs) = msg_cbs_map.get(&msg) {
				for (i, cb) in msg_cbs.into_iter().enumerate() {
					match cb(hwnd, msg, wparam, lparam) {
						Ok(res) => if let Some(value) = res { return LRESULT(value); }
						Err(err) => display_err(format!("on_message([{i}] 0x{msg:X}): {err}"))
					}
				}
			}
			drop(msg_cbs_map);
			return def_win_proc(hwnd, msg, wparam, lparam);
		}
	}
	
	return LRESULT(0);
	
	fn def_win_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
		unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
	}
}

fn display_err(msg: impl fmt::Display) {
	// TODO: display with a window
	println!("{msg}");
}
