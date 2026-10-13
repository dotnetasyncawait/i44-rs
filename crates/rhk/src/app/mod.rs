pub(super) mod main_win;
pub mod tray_icon;
pub mod clipboard;

use super::common::error::{Win32ErrResExt, OsError, Error, OK};
use super::input::{handler::{self, Handler}, hotkey::Hotkey, mods::Mods, keys::Key, hotstr::Hotstr};
use tray_icon::{IconBuilder, TrayIcon, IconEvent};
use main_win::{MainWindow, OnMsgCallback, OnExitCallback};
use std::{process, sync::{OnceLock, Mutex, mpsc}, sync::atomic::{AtomicBool, Ordering}};
use windows::core::{Owned, w};
use windows::Win32::{
	System::Threading::CreateMutexW,
	Foundation::{HWND, POINT, LPARAM, WPARAM, ERROR_ALREADY_EXISTS, GetLastError},
	UI::WindowsAndMessaging::{AppendMenuW, CreatePopupMenu, TrackPopupMenuEx, GetCursorPos, MF_STRING, MF_CHECKED,
		TPM_BOTTOMALIGN, TPM_RETURNCMD, PostThreadMessageW, SetForegroundWindow, WM_QUIT, MessageBoxW, MB_ICONERROR},
	System::Com::{COINIT_MULTITHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx}};

static ICON_SYNC: Mutex<()> = Mutex::new(());
static ICON: OnceLock<TrayIcon> = OnceLock::new();
static WIN: OnceLock<MainWindow> = OnceLock::new();
static IS_HOOKED: AtomicBool = AtomicBool::new(false);
static SUSP_SENDER: OnceLock<mpsc::Sender<u8>> = OnceLock::new();

pub struct App {
	h: Option<Handler>,
	susp_cbs: Vec<fn(bool, bool) -> Result<(), Error>>,
}

pub fn new() -> App {
	set_panic_hook();
	ensure_singleton();
	unsafe { CoInitializeEx(None, COINIT_MULTITHREADED | COINIT_DISABLE_OLE1DDE).unwrap(); }
	
	let win = MainWindow::new();
	
	let mut dir = std::env::current_dir().unwrap();
	dir.push("media");
	
	let icon = win.icon_builder()
		.add(dir.join("default.ico"),   "rhk").expect("failed to add default icon")
		.add(dir.join("suspended.ico"), "rhk (suspended)").expect("failed to add suspended icon")
		.handler(icon_handler)
		.build()
		.expect("failed to build app icon");
	
	icon.display(handler::is_suspended() as _).unwrap();
	ICON.set(icon).expect("icon should not be set");
	
	WIN.set(win).expect("WIN should not be set");
	
	crate::apps::explorer::init();
	
	App { h: Some(Handler::new()), susp_cbs: Vec::new() }
}

pub fn hwnd() -> HWND {
	WIN.get().map(|win| win.hwnd).unwrap_or_default()
}

impl App {
	pub fn hotkey(mut self, mods: Mods, key: Key, f: fn() -> Result<Hotkey, Error>) -> Self {
		self.h.as_mut().unwrap().hotkey(mods, key, f, false);
		self
	}
	
	pub fn hotkey_exempt(mut self, mods: Mods, key: Key, f: fn() -> Result<Hotkey, Error>) -> Self {
		self.h.as_mut().unwrap().hotkey(mods, key, f, true);
		self
	}
	
	pub fn hotstr(mut self, entry: &str, f: fn() -> Result<Hotstr, Error>) -> Self {
		self.h.as_mut().unwrap().hotstr(entry, f, false);
		self
	}
	
	pub fn hotstr_exempt(mut self, entry: &str, f: fn() -> Result<Hotstr, Error>) -> Self {
		self.h.as_mut().unwrap().hotstr(entry, f, true);
		self
	}
	
	pub fn on_exit(self, f: OnExitCallback) -> Self {
		get_win().on_exit(f);
		self
	}
	
	pub fn on_message(self, msg: u32, f: OnMsgCallback) -> Self {
		get_win().on_message(msg, f);
		self
	}
	
	pub fn on_suspend(mut self, cb: fn(bool, bool) -> Result<(), Error>) -> Self {
		self.susp_cbs.push(cb);
		self
	}
	
	pub fn run(mut self) -> ! {
		init_on_suspend_callbacks(self.susp_cbs);
		
		IS_HOOKED.store(true, Ordering::Relaxed);
		let (h_jh, h_thread_id) = self.h.take().unwrap().start(); // start keybd and mouse hooks
		
		get_win().wait();
		
		unsafe { PostThreadMessageW(h_thread_id, WM_QUIT, WPARAM(0), LPARAM(0)).unwrap(); }
		h_jh.join().unwrap();
		
		println!("graceful shutdown");
		process::exit(0);
	}
}

pub fn exit() {
	get_win().exit();
}

pub fn suspend(state: bool) -> Result<(), OsError> {
	let _g = ICON_SYNC.lock().unwrap();
	
	handler::suspend(state);
	if let Some(icon) = ICON.get() {
		icon.display(state as _)?;
	}
	if let Some(sender) = SUSP_SENDER.get() {
		sender.send(((state as u8) << 1) | false as u8).unwrap();
	}
	
	Ok(())
}

pub fn suspend_tgl() -> Result<bool, OsError> {
	suspend_tgl_inner(false)
}

fn suspend_tgl_inner(is_menu: bool) -> Result<bool, OsError> {
	let _g = ICON_SYNC.lock().unwrap();
	
	let state = handler::suspend_tgl();
	if let Some(icon) = ICON.get() {
		icon.display(state as _)?;
	}
	if let Some(sender) = SUSP_SENDER.get() {
		sender.send(((state as u8) << 1) | is_menu as u8).unwrap();
	}
	
	Ok(state)
}

pub fn icon_builder() -> IconBuilder {
	get_win().icon_builder()
}

fn get_win() -> &'static MainWindow {
	WIN.get().expect("WIN should be set")
}

fn icon_handler(icon: &TrayIcon, event: IconEvent) -> Result<(), Error> {
	if event != IconEvent::RClick {
		return OK;
	}
	
	const SUSPEND: i32 = 1;
	const EXIT: i32 = 2;
	
	let res = unsafe {
		if !SetForegroundWindow(icon.hwnd()).as_bool() {
			return OK;
		}
		
		let menu = Owned::new(CreatePopupMenu().context("failed to create Popup menu")?);
		
		let mut susp_flags = MF_STRING;
		if handler::is_suspended() {
			susp_flags |= MF_CHECKED;
		}
		
		AppendMenuW(*menu, susp_flags, SUSPEND as _, w!("Suspend")).context("failed to append 'Suspend' menu item")?;
		AppendMenuW(*menu, MF_STRING, EXIT as _, w!("Exit")).context("failed to append 'Exit' menu item")?;
		
		let mut point = POINT::default();
		GetCursorPos(&mut point).context("failed to get cursor position")?;
		
		TrackPopupMenuEx(*menu, (TPM_BOTTOMALIGN | TPM_RETURNCMD).0, point.x, point.y, hwnd(), None)
	};
	
	match res.0 {
		0 => {}, // cancelled
		SUSPEND => _ = suspend_tgl_inner(true)?,
		EXIT => exit(),
		_ => unreachable!()
	}
	
	OK
}

fn init_on_suspend_callbacks(cbs: Vec<fn(bool, bool) -> Result<(), Error>>) {
	let (tx, rx) = mpsc::channel::<u8>();
	
	let _ = std::thread::spawn(move || {
		for msg in rx {
			let (state, is_menu) = ((msg & 2) != 0, (msg & 1) != 0);
			
			for cb in &cbs {
				if let Err(err) = cb(state, is_menu) {
					display_err(format!("on_suspend: {err}"));
				}
			}
		}
	});
	
	SUSP_SENDER.set(tx).expect("SUSP_SENDER should only be set once");
}

fn display_err(err: impl std::fmt::Display) {
	// TODO: display with a window
	println!("{err}");
}

fn set_panic_hook() {
	std::panic::set_hook(Box::new(|info| {
		let loc = info.location().unwrap();
		
		let text = format!(
			"panic at {}:{}:{}: {}",
			loc.file(), loc.line(), loc.column(), info.payload_as_str().unwrap_or_default());
		
		if IS_HOOKED.load(Ordering::Relaxed) {
			// TODO: log errors occurred while input-hooks are set and exit as quick as possible.
			// Otherwise, if the panicking thread is a hook-thread, all the input events will freeze
			// until each event times out, or OS finally removes the hook.
			println!("{text}");
		} else {
			let text = text
				.encode_utf16()
				.chain(std::iter::once(0))
				.collect::<Box<[u16]>>();
			
			_ = unsafe { MessageBoxW(Some(hwnd()), windows_core::PCWSTR(text.as_ptr()), w!("Error"), MB_ICONERROR) };
		}
		
		process::exit(0);
	}));
}

fn ensure_singleton() {
	unsafe {
		let _mutex = CreateMutexW(None, false, w!("Global\\rhk_mtx")).expect("failed to create a named mutex");
		if GetLastError() == ERROR_ALREADY_EXISTS {
			panic!("Application is already running");
		}
	}
}
