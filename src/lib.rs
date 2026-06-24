pub mod input;
pub mod misc;
pub mod hid;
pub mod common;
pub mod apps;

use common::error::Error;
use input::{handler::{self, Handler}, hotkey::Hotkey, mods::Mods, keys::Key};
use misc::{main_win::{MainWindow, MsgHandler, Icon}, tray_icon::{IconBuilder, TrayIcon, IconEvent}};
use std::{env, process, sync::OnceLock};
use windows::core::{Owned, w};
use windows::Win32::{
	Foundation::{HWND, POINT},
	UI::WindowsAndMessaging::{AppendMenuW, CreatePopupMenu, TrackPopupMenuEx, GetCursorPos, MF_STRING, MF_CHECKED,
		TPM_BOTTOMALIGN, TPM_RETURNCMD}};

static ICON: OnceLock<Icon> = OnceLock::new();

#[derive(Clone, Copy, Debug)]
pub enum ExitReason {
	Shutdown,
	Restart
}

pub struct App {
	h: Option<Handler>,
	win: MainWindow,
	on_exit: Vec<fn(ExitReason)>,
}

impl App {
	pub fn new() -> Self {
		let win = MainWindow::new();
		
		let mut dir = std::env::current_dir().unwrap();
		dir.push("media");
		
		let icon = win.icon_builder()
			.add("i44",             dir.join("default.ico")).expect("failed to add 'default' icon")
			.add("i44 (suspended)", dir.join("suspended.ico")).expect("failed to add 'suspended' icon")
			.handler(icon_handler)
			.build();
		
		let icon = win.add_icon(icon);
		ICON.set(icon).expect("icon should not be set");
		
		Self { h: Some(Handler::new()), win, on_exit: Vec::default() }
	}
	
	pub fn hotkey(mut self, mods: Mods, key: Key, f: fn() -> Result<Hotkey, Error>) -> Self {
		self.h.as_mut().unwrap().hotkey(mods, key, f);
		self
	}
	
	pub fn hotkey_exempt(mut self, mods: Mods, key: Key, f: fn() -> Result<Hotkey, Error>) -> Self {
		self.h.as_mut().unwrap().hotkey_exempt(mods, key, f);
		self
	}
	
	pub fn on_exit(mut self, f: fn(ExitReason)) -> Self {
		self.on_exit.push(f);
		self
	}
	
	pub fn on_message(self, msg: u32, f: MsgHandler) -> Self {
		self.win.on_message(msg, f);
		self
	}
	
	pub fn icon_builder(&self) -> IconBuilder {
		self.win.icon_builder()
	}

	pub fn add_icon(&self, icon: TrayIcon) -> Icon {
		self.win.add_icon(icon)
	}
	
	pub fn run(mut self) -> ! {
		ICON.get().unwrap().display(handler::is_suspended() as _).unwrap();
		
		self.h.take().unwrap().start();
		
		let exit_reason = match handler::wait() {
			0 => ExitReason::Shutdown,
			1 => ExitReason::Restart,
			_ => unreachable!()
		};
		
		for f in self.on_exit {
			f(exit_reason);
		}
		
		self.win.exit();
		
		if let ExitReason::Restart = exit_reason {
			let exe = env::current_exe().unwrap();
			let _ = process::Command::new(exe)
				.args(env::args().skip(1))
				.spawn()
				.expect("failed to launch itself");
		}
		
		process::exit(0);
	}
	
	pub fn hwnd(&self) -> HWND {
		self.win.hwnd
	}
	
	pub fn exit() {
		handler::exit(0);
	}
	
	pub fn suspend(state: bool) {
		handler::suspend(state);
		if let Some(icon) = ICON.get() {
			icon.display(state as _).expect("icon should not outlive App");
		}
	}
	
	pub fn suspend_togg() -> bool {
		let state = handler::suspend_togg();
		if let Some(icon) = ICON.get() {
			icon.display(state as _).expect("icon should not outlive App");
		}
		state
	}
	
	pub fn restart() {
		handler::exit(1);
	}
}

fn icon_handler(icon: &TrayIcon, event: IconEvent) {
	if event != IconEvent::RClick {
		return;
	}
	
	const SUSPEND: i32 = 1;
	const EXIT: i32 = 2;
	
	let res = unsafe {
		let menu = Owned::new(CreatePopupMenu().expect("failed to create Popup menu"));
		
		let mut susp_flags = MF_STRING;
		if handler::is_suspended() {
			susp_flags |= MF_CHECKED;
		}
		
		AppendMenuW(*menu, susp_flags, SUSPEND as _, w!("Suspend")).unwrap();
		AppendMenuW(*menu, MF_STRING, EXIT as _, w!("Exit")).unwrap();
		
		let mut point = POINT::default();
		GetCursorPos(&mut point).unwrap();
		
		TrackPopupMenuEx(*menu, (TPM_BOTTOMALIGN | TPM_RETURNCMD).0, point.x, point.y, icon.hwnd(), None)
	};
	
	match res.0 {
		0 => {}, // cancelled
		SUSPEND => icon.display(handler::suspend_togg() as _).unwrap(),
		EXIT => App::exit(),
		_ => unreachable!()
	}
}