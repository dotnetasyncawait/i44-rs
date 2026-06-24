use std::{ffi::c_void, ptr, sync::{Arc, Mutex, Weak, atomic::{AtomicBool, Ordering::Relaxed}, mpsc}};
use std::{thread::{self, JoinHandle}, collections::{HashMap, hash_map::Entry}};
use super::tray_icon::{TrayIcon, IconBuilder, IconEvent};
use crate::common::error::Error;
use windows::core::w;
use windows::Win32::{
	Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
	System::LibraryLoader::GetModuleHandleW,
	UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW,
		TranslateMessage, WM_QUIT, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP, CREATESTRUCTW, GWLP_USERDATA, GetWindowLongPtrW,
		SetWindowLongPtrW, WM_NCCREATE, WM_USER, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN,
		SetForegroundWindow, WM_ENTERIDLE, WM_ENTERMENULOOP, WM_EXITMENULOOP, EndMenu, PostMessageW, PostQuitMessage}};

pub type MsgHandler = fn(HWND, u32, WPARAM, LPARAM) -> isize;
type Win32Error = windows::core::Error;

const WM_ICON_MSG: u32 = WM_USER | 0xFF;

pub struct State {
	handlers: Mutex<HashMap<u32, Vec<MsgHandler>>>,
	icons: Mutex<HashMap<u32, Arc<TrayIcon>>>,
	is_menu_loop: AtomicBool,
}

impl State {
	pub fn new() -> Self {
		Self { handlers: Mutex::new(HashMap::new()), icons: Mutex::new(HashMap::new()), is_menu_loop: AtomicBool::new(false) }
	}
}

#[allow(unused)]
pub struct MainWindow {
	pub hwnd: HWND,
	jh: JoinHandle<()>,
	
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
		
		Self { jh, hwnd, state: weak_state }
	}
	
	pub fn on_message(&self, msg: u32, f: MsgHandler) {
		let state = self.state.upgrade().expect("State should be alive if Self is");
		let mut map = state.handlers.lock().unwrap();
		
		match map.entry(msg) {
			Entry::Occupied(mut occupied) => occupied.get_mut().push(f),
			Entry::Vacant(vacant) => _ = vacant.insert(vec![f]),
		}
	}
	
	pub fn icon_builder(&self) -> IconBuilder {
		IconBuilder::new(self.hwnd, WM_ICON_MSG)
	}
	
	pub fn add_icon(&self, icon: TrayIcon) -> Icon {
		let state = self.state.upgrade().expect("State should be alive if Self is");
		let id = icon.id();
		
		let icon = Arc::new(icon);
		let weak_icon = Arc::downgrade(&icon);
		
		if state.icons.lock().unwrap().insert(id, icon).is_some() {
			panic!("Duplicate icon with id: {id}")
		}
		
		Icon { id, state: Arc::downgrade(&state), inner: weak_icon }
	}
	
	pub fn exit(self) {
		// Note: the reason for not posting a quit message directly to the thread (via PostThreadMessage)
		// is that if a menu loop is running, the thread message will be ignored.
		unsafe { PostMessageW(Some(self.hwnd), WM_QUIT, WPARAM(0), LPARAM(0)).unwrap() }
		self.jh.join().unwrap();
	}
}

fn win_mq(tx: mpsc::Sender<usize>, state: Arc<State>) {
	let h_inst = HINSTANCE(unsafe { GetModuleHandleW(None).unwrap().0 });
	let class_name = w!("i44_win");
	
	let mut wc = WNDCLASSW::default();
	wc.lpfnWndProc = Some(win_proc);
	wc.hInstance = h_inst;
	wc.lpszClassName = class_name;
	
	if unsafe { RegisterClassW(&wc) } == 0 {
		panic!("failed to register class: {}", Win32Error::from_thread().message());
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
	
	_ = unsafe { Arc::from_raw(r_state) };
}

unsafe extern "system" fn win_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	match msg {
		WM_NCCREATE => {
			let cs = unsafe { ptr::read(lparam.0 as *const CREATESTRUCTW) };
			unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize) };
			return LRESULT(1);
		},
		WM_ICON_MSG if lparam.0 as u32 == WM_MOUSEMOVE => return LRESULT(0),
		WM_ENTERIDLE => return def_win_proc(hwnd, msg, wparam, lparam),
		_ => {}
	}
	
	let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State };
	if ptr.is_null() {
		return def_win_proc(hwnd, msg, wparam, lparam);
	}
	
	let state = unsafe { &*ptr };
	
	match msg {
		WM_ENTERMENULOOP => {
			_ = unsafe { SetForegroundWindow(hwnd) };
			state.is_menu_loop.store(true, Relaxed);
			return def_win_proc(hwnd, msg, wparam, lparam);
		},
		WM_EXITMENULOOP => {
			state.is_menu_loop.store(false, Relaxed);
			return def_win_proc(hwnd, msg, wparam, lparam);
		},
		WM_QUIT => {
			if state.is_menu_loop.load(Relaxed) {
				unsafe { EndMenu().expect("failed to end menu") };
			}
			unsafe { PostQuitMessage(0) };
			return LRESULT(0);
		},
		WM_ICON_MSG => {
			let event = match lparam.0 as u32 {
				WM_LBUTTONDOWN => IconEvent::LClick,
				WM_LBUTTONDBLCLK => IconEvent::DClick,
				WM_RBUTTONDOWN => IconEvent::RClick,
				_ => return LRESULT(0)
			};
			
			let icons = state.icons.lock().unwrap();
			let icon_id = wparam.0 as u32;
			
			if let Some(icon) = icons.get(&icon_id) && let Some(f) = icon.handler {
				f(icon, event);
			}
			
			return LRESULT(0);
		},
		_ => {}
	}
	
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
	return def_win_proc(hwnd, msg, wparam, lparam);
	
	fn def_win_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
		unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
	}
}

#[derive(Debug)]
pub struct Icon {
	id: u32,
	state: Weak<State>,
	inner: Weak<TrayIcon>,
}

impl Icon {
	pub fn display(&self, index: usize) -> Result<(), Error> {
		if let Some(icon) = self.inner.upgrade() {
			icon.display(index)
		} else {
			Self::dropped_err()
		}
	}
	
	pub fn show(&self) -> Result<(), Error> {
		if let Some(icon) = self.inner.upgrade() {
			icon.show()
		} else {
			Self::dropped_err()
		}
	}
	
	pub fn hide(&self) -> Result<(), Error> {
		if let Some(icon) = self.inner.upgrade() {
			icon.hide()
		} else {
			Self::dropped_err()
		}
	}
	
	pub fn toggle_visibility(&self) -> Result<bool, Error> {
		if let Some(icon) = self.inner.upgrade() {
			icon.toggle_visibility()
		} else {
			Self::dropped_err()
		}
	}
	
	fn dropped_err<T>() -> Result<T, Error> {
		Err(Error::Other(String::from("Icon outlived App")))
	}
}

impl Drop for Icon {
	fn drop(&mut self) {
		if let Some(state) = self.state.upgrade() {
			state.icons.lock().unwrap().remove(&self.id).expect("Icon must be present");
		}
	}
}