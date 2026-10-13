use std::{fmt, os::windows::ffi::OsStrExt, path::Path};
use std::sync::{Arc, Weak, Mutex, MutexGuard, atomic::{AtomicU32, Ordering}};
use crate::common::error::{OsError, Win32ErrResExt, Error};
use super::main_win::{self, State as WinState};
use windows::core::{Owned, PCWSTR};
use windows::Win32::{
	Foundation::HWND,
	UI::Shell::{NIM_ADD, NIM_MODIFY, NIM_DELETE, NIF_MESSAGE, NIF_STATE, NIF_ICON, NIF_TIP, NIS_HIDDEN,
		NOTIFYICONDATAW, Shell_NotifyIconW, NOTIFY_ICON_DATA_FLAGS, NOTIFY_ICON_STATE},
	UI::WindowsAndMessaging::{HICON, IMAGE_ICON, LR_LOADFROMFILE, LoadImageW}};

type EventHandler = fn(&TrayIcon, IconEvent) -> Result<(), Error>;

#[derive(PartialEq, Eq)]
pub enum IconEvent {
	LClick,
	RClick,
	DClick,
}

#[derive(Debug)]
pub struct TrayIcon {
	state: Arc<State>,
	pub(super) handler: Option<EventHandler>
}

impl TrayIcon {
	fn new(
		hwnd: HWND,
		icons: Box<[Item]>,
		handler: Option<EventHandler>,
		win_state: Weak<WinState>) -> Result<Self, OsError>
	{
		static NEXT_ID: AtomicU32 = AtomicU32::new(0);
		let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
		
		let nid = NOTIFYICONDATAW {
			cbSize: size_of::<NOTIFYICONDATAW>() as _,
			hWnd: hwnd,
			uID: id,
			uFlags: NIF_MESSAGE | NIF_STATE,
			uCallbackMessage: main_win::WM_ICON_MSG,
			dwState: NIS_HIDDEN,
			dwStateMask: NIS_HIDDEN,
			..Default::default()
		};
		
		unsafe { Shell_NotifyIconW(NIM_ADD, &nid).ok().context("failed to create icon")?; }
		
		let handler = Some(handler.unwrap_or(|_, _| Ok(())));
		let inner = Mutex::new(Inner { nid, icons, index: usize::MAX, visible: false });
		let state = Arc::new(State { id, hwnd, inner, win_state });
		
		Ok(TrayIcon { state, handler })
	}
	
	fn new_outer(s: &Self) -> Self {
		TrayIcon { state: Arc::clone(&s.state), handler: None }
	}
	
	pub fn id(&self) -> u32 { self.state.id }
	pub fn hwnd(&self) -> HWND { self.state.hwnd }
	pub fn display(&self, index: usize) -> Result<(), OsError> { self.lock().display(index) }
	pub fn show(&self) -> Result<(), OsError> { self.lock().show() }
	pub fn hide(&self) -> Result<(), OsError> { self.lock().hide() }
	pub fn toggle_visibility(&self) -> Result<bool, OsError> { self.lock().toggle_visibility() }
	pub fn update_tip(&self, index: usize, tip: impl AsRef<str>) -> Result<(), OsError> {
		self.lock().update_tip(index, tip.as_ref())
	}
	
	fn lock(&self) -> MutexGuard<'_, Inner> { self.state.inner.lock().unwrap() }
}

impl Drop for TrayIcon {
	fn drop(&mut self) {
		if self.handler.is_some() {
			unsafe { Shell_NotifyIconW(NIM_DELETE, &self.lock().nid).expect("failed to delete icon"); }
		} else if let Some(state) = self.state.win_state.upgrade() {
			state.icons.lock().unwrap().remove(&self.id()).expect("icon should be present");
		}
	}
}

#[derive(Debug)]
struct State {
	id: u32,
	hwnd: HWND,
	inner: Mutex<Inner>,
	win_state: Weak<WinState>,
}

// SAFETY: due to Self::hwnd and Self::inner.nid
unsafe impl Send for State {}
unsafe impl Sync for State {}

struct Item {
	tip: Box<[u16]>,
	h_icon: Owned<HICON>,
}

struct Inner {
	nid: NOTIFYICONDATAW,
	index: usize,
	visible: bool,
	icons: Box<[Item]>,
}

impl Inner {
	fn display(&mut self, index: usize) -> Result<(), OsError> {
		assert!(index < self.icons.len(), "invalid icon index");
		
		if index == self.index {
			return self.show();
		}
		
		let flags = NIF_ICON | NIF_TIP | (if !self.visible { NIF_STATE } else { NOTIFY_ICON_DATA_FLAGS(0) });
		self.notify(index, flags, false)?;
		
		self.visible = true;
		self.index = index;
		
		Ok(())
	}
	
	fn show(&mut self) -> Result<(), OsError> {
		if !self.visible {
			self.toggle_visibility()?;
		}
		Ok(())
	}
	
	fn hide(&mut self) -> Result<(), OsError> {
		if self.visible {
			self.toggle_visibility()?;
		}
		Ok(())
	}
	
	fn toggle_visibility(&mut self) -> Result<bool, OsError> {
		self.notify(0, NIF_STATE, self.visible)?; // Index is ignored for NIF_STATE
		self.visible ^= true;
		Ok(self.visible)
	}
	
	fn update_tip(&mut self, index: usize, tip: &str) -> Result<(), OsError> {
		assert!(index < self.icons.len(), "invalid icon index");
		
		self.icons[index].tip = encode_tip(tip);
		
		if index == self.index {
			self.notify(index, NIF_TIP, false)
		} else {
			Ok(())
		}
	}
	
	fn notify(&mut self, index: usize, flags: NOTIFY_ICON_DATA_FLAGS, hide: bool) -> Result<(), OsError> {
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
		
		unsafe { Shell_NotifyIconW(NIM_MODIFY, &self.nid) }
			.ok()
			.context("failed to notify icon")
	}
}

impl fmt::Debug for Inner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Inner")
			.field("id", &self.nid.uID)
			.field("hwnd", &self.nid.hWnd)
			.field("index", &self.index)
			.field("visible", &self.visible)
			.finish()
	}
}

pub struct IconBuilder {
	hwnd: HWND,
	icons: Vec<Item>,
	handler: Option<EventHandler>,
	win_state: Weak<WinState>,
}

impl IconBuilder {
	pub(super) fn new(hwnd: HWND, win_state: Weak<WinState>) -> Self {
		Self { hwnd, icons: Vec::new(), handler: None, win_state }
	}
	
	pub fn add(mut self, path: impl AsRef<Path>, tip: impl AsRef<str>) -> Result<Self, OsError> {
		use std::iter::once;
		
		let path = path
			.as_ref()
			.as_os_str()
			.encode_wide()
			.chain(once(0))
			.collect::<Vec<u16>>();
		
		let handle = unsafe { LoadImageW(
			None,
			PCWSTR(path.as_ptr()),
			IMAGE_ICON,
			16, 16,
			LR_LOADFROMFILE).context("failed to load icon image")?
		};
		
		let item = Item { tip: encode_tip(tip.as_ref()), h_icon: unsafe { Owned::new(HICON(handle.0)) } };
		self.icons.push(item);
		
		Ok(self)
	}
	
	pub fn handler(mut self, h: EventHandler) -> Self {
		self.handler = Some(h);
		self
	}
	
	pub fn build(self) -> Result<TrayIcon, OsError> {
		let Some(win_state) = self.win_state.upgrade() else {
			// The app is being shut down, so it makes no difference what error is returned.
			// Nonetheless, add some diagnostic value if we happen to see the error.
			return Err(OsError::from_hr("IconBuilder::build()", windows_core::HRESULT(0)))
		};
		
		let icon = TrayIcon::new(self.hwnd, self.icons.into_boxed_slice(), self.handler, self.win_state)?;
		let icon_outer = TrayIcon::new_outer(&icon);
		let id = icon.id();
		
		let inserted = win_state.icons.lock().unwrap().insert(id, icon).is_none();
		assert!(inserted, "duplicate icon with id: {id}");
		
		Ok(icon_outer)
	}
}

fn encode_tip(tip: &str) -> Box<[u16]> {
	tip
		.encode_utf16()
		.take(127) // szTip member of NOTIFYICONDATAW is limited to 127 wide-characters + 1 NULL
		.chain(std::iter::once(0)) // 128-th NULL-character
		.collect()
}
