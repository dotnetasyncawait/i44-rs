pub(super) mod tray_icon;
pub(super) mod main_win;

use super::common::error::{Error, ErrResultExt, OK};
use super::input::{handler::{self, Handler}, hotkey::Hotkey, mods::Mods, keys::Key};
use tray_icon::{IconBuilder, TrayIcon, IconEvent};
use main_win::{MainWindow, OnMsgCallback, OnExitCallback, Icon};
use std::{process, sync::{OnceLock, Mutex}};
use windows::core::{Owned, w};
use windows::Win32::{
	Foundation::{HWND, POINT, LPARAM, WPARAM},
	UI::WindowsAndMessaging::{AppendMenuW, CreatePopupMenu, TrackPopupMenuEx, GetCursorPos, MF_STRING, MF_CHECKED,
		TPM_BOTTOMALIGN, TPM_RETURNCMD, PostThreadMessageW, SetForegroundWindow, WM_QUIT}};

static ICON: OnceLock<Icon> = OnceLock::new();
static WIN: OnceLock<MainWindow> = OnceLock::new();

pub struct App {
	h: Option<Handler>
}

pub fn new() -> App {
	let win = MainWindow::new();
	
	let mut dir = std::env::current_dir().unwrap();
	dir.push("media");
	
	let icon = win.icon_builder()
		.add("i44",             dir.join("default.ico")).expect("failed to add 'default' icon")
		.add("i44 (suspended)", dir.join("suspended.ico")).expect("failed to add 'suspended' icon")
		.handler(icon_handler)
		.build();
	
	icon.display(handler::is_suspended() as _).unwrap();
	ICON.set(win.add_icon(icon)).expect("icon should not be set");
	
	WIN.set(win).expect("WIN should not be set");
	App { h: Some(Handler::new()) }
}

pub fn hwnd() -> HWND {
	WIN.get().map(|win| win.hwnd).unwrap_or_default()
}

impl App {
	pub fn hotkey(mut self, mods: Mods, key: Key, f: fn() -> Result<Hotkey, Error>) -> Self {
		self.h.as_mut().unwrap().hotkey(mods, key, f);
		self
	}
	
	pub fn hotkey_exempt(mut self, mods: Mods, key: Key, f: fn() -> Result<Hotkey, Error>) -> Self {
		self.h.as_mut().unwrap().hotkey_exempt(mods, key, f);
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
	
	pub fn run(mut self) -> ! {
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

static STATE_SYNC: Mutex<()> = Mutex::new(());

pub fn suspend(state: bool) {
	let _g = STATE_SYNC.lock().unwrap();
	
	handler::suspend(state);
	if let Some(icon) = ICON.get() {
		icon.display(state as _).unwrap();
	}
}

pub fn suspend_tgl() -> bool {
	let _g = STATE_SYNC.lock().unwrap();
	
	let state = handler::suspend_tgl();
	if let Some(icon) = ICON.get() {
		icon.display(state as _).unwrap();
	}
	state
}

pub fn icon_builder() -> IconBuilder {
	get_win().icon_builder()
}

pub fn add_icon(icon: TrayIcon) -> Icon {
	get_win().add_icon(icon)
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
		
		let menu = Owned::new(CreatePopupMenu().with_context(|| "failed to create Popup menu")?);
		
		let mut susp_flags = MF_STRING;
		if handler::is_suspended() {
			susp_flags |= MF_CHECKED;
		}
		
		AppendMenuW(*menu, susp_flags, SUSPEND as _, w!("Suspend")).with_context(|| "failed to append 'Suspend' menu item")?;
		AppendMenuW(*menu, MF_STRING, EXIT as _, w!("Exit")).with_context(|| "failed to append 'Exit' menu item")?;
		
		let mut point = POINT::default();
		GetCursorPos(&mut point).with_context(|| "failed to get cursor position")?;
		
		TrackPopupMenuEx(*menu, (TPM_BOTTOMALIGN | TPM_RETURNCMD).0, point.x, point.y, icon.hwnd(), None)
	};
	
	match res.0 {
		0 => {}, // cancelled
		SUSPEND => return icon.display(handler::suspend_tgl() as _),
		EXIT => self::exit(),
		_ => unreachable!()
	};
	
	OK
}