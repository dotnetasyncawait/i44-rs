use std::{path::Path, sync::atomic::{AtomicU32, Ordering::Relaxed}};
use crate::common::error::{Error, Win32ErrExt, ErrResultExt};
use windows::core::{Owned, PCWSTR};
use windows::Win32::{
	Foundation::HWND,
	UI::Shell::{NIM_ADD, NIM_MODIFY, NIM_DELETE, NIF_MESSAGE, NIF_STATE, NIF_ICON, NIF_TIP, NIS_HIDDEN,
		NOTIFYICONDATAW, Shell_NotifyIconW, NOTIFY_ICON_DATA_FLAGS, NOTIFY_ICON_STATE},
	UI::WindowsAndMessaging::{HICON, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadImageW}};

type EventHandler = fn(&mut TrayIcon, IconEvent);
type Win32Error = windows::core::Error;

pub enum IconEvent {
	LClick,
	RClick,
	DClick,
}

struct Item {
	tip: Vec<u16>,
	h_icon: Owned<HICON>,
}

pub struct TrayIcon {
	nid: NOTIFYICONDATAW,
	index: usize,
	visible: bool,
	icons: Vec<Item>,
	pub(super) handler: Option<EventHandler>
}

impl TrayIcon {
	fn new(hwnd: HWND, msg: u32, icons: Vec<Item>, handler: Option<EventHandler>) -> Self {
		static NEXT_ID: AtomicU32 = AtomicU32::new(0);
		
		let nid = NOTIFYICONDATAW {
			cbSize: size_of::<NOTIFYICONDATAW>() as _,
			hWnd: hwnd,
			uID: NEXT_ID.fetch_add(1, Relaxed),
			uFlags: NIF_MESSAGE | NIF_STATE,
			uCallbackMessage: msg,
			dwState: NIS_HIDDEN,
			dwStateMask: NIS_HIDDEN,
			..Default::default()
		};
		
		let succeeded = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
		assert!(succeeded, "Icon creation failed: {:?}", Win32Error::from_thread());
		
		TrayIcon { nid, icons, handler, index: usize::MAX, visible: false }
	}
	
	pub(super) fn id(&self) -> u32 { self.nid.uID }
	
	pub fn display(&mut self, index: usize) -> Result<(), Error> {
		assert!(index < self.icons.len(), "Invalid icon index");
		
		if index == self.index {
			return self.show();
		}
		
		let flags = NIF_ICON | NIF_TIP | (if !self.visible { NIF_STATE } else { NOTIFY_ICON_DATA_FLAGS(0) });
		self.notify(index, flags, false)?;
		
		self.visible = true;
		self.index = index;
		
		Ok(())
	}
	
	pub fn show(&mut self) -> Result<(), Error> {
		if !self.visible {
			self.toggle_visibility()?;
		}
		Ok(())
	}
	
	pub fn hide(&mut self) -> Result<(), Error> {
		if self.visible {
			self.toggle_visibility()?;
		}
		Ok(())
	}
	
	pub fn toggle_visibility(&mut self) -> Result<bool, Error> {
		self.notify(0, NIF_STATE, self.visible)?; // Index is ignored for NIF_STATE
		self.visible ^= true;
		Ok(self.visible)
	}
	
	fn notify(&mut self, index: usize, flags: NOTIFY_ICON_DATA_FLAGS, hide: bool) -> Result<(), Error> {
		if flags.contains(NIF_ICON) {
			self.nid.hIcon = *self.icons[index].h_icon;
		}
		
		if flags.contains(NIF_TIP) {
			let tip = &self.icons[index].tip;
			self.nid.szTip[0..tip.len()].copy_from_slice(tip);
		}
		
		if flags.contains(NIF_STATE) {
			self.nid.dwState = NOTIFY_ICON_STATE(hide as u32);
			self.nid.dwStateMask = NIS_HIDDEN;
		}
		
		self.nid.uFlags = flags;
		
		if unsafe { Shell_NotifyIconW(NIM_MODIFY, &self.nid).as_bool() } {
			Ok(())
		} else {
			Err(Error::Win32(Win32Error::from_thread().with_context("Failed to update icon")))
		}
	}
}

impl Drop for TrayIcon {
	fn drop(&mut self) {
		let deleted = unsafe { Shell_NotifyIconW(NIM_DELETE, &self.nid).as_bool() };
		assert!(deleted, "Failed to delete icon: {:?}", Win32Error::from_thread())
	}
}

unsafe impl Send for TrayIcon {} // I'm scared here

pub struct IconBuilder {
	hwnd: HWND,
	msg: u32,
	icons: Vec<Item>,
	handler: Option<EventHandler>,
}

impl IconBuilder {
	pub fn new(hwnd: HWND, msg: u32) -> Self {
		Self { hwnd, msg, icons: Vec::new(), handler: None }
	}
	
	pub fn add<P: AsRef<Path>>(mut self, tip: &'_ str, path: P) -> Result<Self, Error> {
		use std::iter::once;
		
		let path = path
			.as_ref()
			.to_str()
			.ok_or(Error::Other(String::from("Invalid path")))?
			.encode_utf16()
			.chain(once(0))
			.collect::<Vec<u16>>();
		
		let handle = unsafe {
			LoadImageW(None, PCWSTR(path.as_ptr()), IMAGE_ICON, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE)
				.with_context(|| String::from("Failed to load icon"))?
		};
		
		let h_icon = unsafe { Owned::new(HICON(handle.0)) };
		let tip = tip
			.encode_utf16()
			.take(127)      // szTip member of NOTIFYICONDATAW is limited to 127 wide-characters + 1 NULL
			.chain(once(0)) // 128-th character
			.collect::<Vec<u16>>();
		
		self.icons.push(Item { tip, h_icon });
		Ok(self)
	}
	
	pub fn handler(mut self, h: EventHandler) -> Self {
		self.handler = Some(h);
		self
	}
	
	pub fn build(self) -> TrayIcon {
		TrayIcon::new(self.hwnd, self.msg, self.icons, self.handler)
	}
}