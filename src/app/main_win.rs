use std::{cell::RefCell, ffi::c_void, sync::{Arc, Mutex, Weak, mpsc}};
use std::{thread::{self, JoinHandle}, collections::{HashMap, hash_map::Entry}};
use super::tray_icon::{TrayIcon, IconBuilder, IconEvent};
use crate::{common::error::{Error, Win32Error}};
use windows::{Win32::UI::WindowsAndMessaging::{WM_CLOSE, WM_DESTROY, WM_ENDSESSION}, core::w};
use windows::Win32::{
	Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
	System::LibraryLoader::GetModuleHandleW,
	UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW,
		TranslateMessage, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP, CREATESTRUCTW, GWLP_USERDATA, GetWindowLongPtrW,
		SetWindowLongPtrW, WM_NCCREATE, WM_USER, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN,
		PostMessageW, PostQuitMessage}};

pub type OnMsgCallback = fn(HWND, u32, WPARAM, LPARAM) -> Option<isize>;
pub type OnExitCallback = fn() -> bool;

const WM_ICON_MSG: u32 = WM_USER | 0xFF;

pub struct State {
	on_msg_cbs: Mutex<HashMap<u32, Vec<OnMsgCallback>>>,
	on_exit_cbs: Mutex<Vec<OnExitCallback>>,
	icons: Mutex<HashMap<u32, Arc<TrayIcon>>>,
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
		IconBuilder::new(self.hwnd, WM_ICON_MSG)
	}
	
	pub fn add_icon(&self, icon: TrayIcon) -> Icon {
		let state = self.state.upgrade().expect("TODO");
		let id = icon.id();
		
		let icon = Arc::new(icon);
		let weak_icon = Arc::downgrade(&icon);
		
		if state.icons.lock().unwrap().insert(id, icon).is_some() {
			panic!("Duplicate icon with id: {id}")
		}
		
		Icon { id, state: Arc::downgrade(&state), inner: weak_icon }
	}
	
	pub fn wait(&self) {
		self.jh
			.borrow_mut()
			.take().expect("main window must only be awaited once")
			.join().unwrap();
	}
	
	pub fn exit(&self) {
		// TODO: what if the window is already gone?
		unsafe { PostMessageW(Some(self.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap(); }
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
		panic!("failed to register class: {}", Win32Error::from_thread().message());
	}
	
	let hwnd = unsafe { CreateWindowExW(
		WS_EX_TOOLWINDOW,
		class_name,
		None,
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
			unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize) };
			return LRESULT(1);
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
			
			_ = unsafe { Arc::from_raw(r_state) };
			return def_win_proc(hwnd, msg, wparam, lparam);
		},
		WM_DESTROY => {
			unsafe { PostQuitMessage(0); }
			return LRESULT(0);
		},
		WM_ENDSESSION if wparam.0 == 1 => {
			let exit_cbs = state.on_exit_cbs.lock().unwrap();
			for f in exit_cbs.iter() {
				if f() {
					break;
				}
			}
			drop(exit_cbs);
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
			
			if let Some(icon) = icons.get(&icon_id)
				&& let Some(f) = icon.handler
				&& let Err(err) = f(icon, event)
			{
				println!("From icon handler: {err:?}; (id: {icon_id})"); // TODO: display with window
			}
			
			return LRESULT(0);
		},
		_ => {}
	}
	
	let msg_cbs_map = state.on_msg_cbs.lock().unwrap();
	if let Some(msg_cbs) = msg_cbs_map.get(&msg) {
		for f in msg_cbs {
			if let Some(res) = f(hwnd, msg, wparam, lparam) {
				return LRESULT(res);
			}
		}
	}
	drop(msg_cbs_map);
	
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