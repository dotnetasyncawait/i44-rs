pub mod input;
pub mod misc;
pub mod hid;
pub mod common;
pub mod apps;

use common::error::Error;
use input::{handler::{self, Handler}, hotkey::Hotkey, mods::Mods, keys::Key};
use misc::{main_win::{MainWindow, MsgHandler, Icon}, tray_icon::{IconBuilder, TrayIcon}};
use windows::Win32::Foundation::HWND;
use std::{env, process};

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
		Self { h: Some(Handler::new()), win: MainWindow::new(), on_exit: Vec::default() }
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
	}
	
	pub fn suspend_togg() -> bool {
		handler::suspend_togg()
	}
	
	pub fn restart() {
		handler::exit(1);
	}
}