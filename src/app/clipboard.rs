use core::fmt;
use std::{iter::once, mem::forget, ptr, slice};
use crate::common::error::{OsError, Win32ErrResExt};
use windows_core::Owned;
use windows::Win32::{
	Foundation::{HANDLE, HGLOBAL, NO_ERROR}, System::Ole,
	System::{
		DataExchange::{
			CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, SetClipboardData},
		Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock}}};

const CF_UNICODETEXT: u32 = Ole::CF_UNICODETEXT.0 as u32;

pub fn get_text() -> Result<String, GetTextError> {
	if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT).is_ok() } {
		open_clipb()?;
		let res = get_clipb_text();
		let close_res = close_clipb();
		Ok(res.and_then(|s| close_res.map(|_| s))?)
	} else {
		Err(GetTextError::FormatUnavailable)
	}
}

fn get_clipb_text() -> Result<String, OsError> {
	unsafe {
		let g_mem = HGLOBAL(GetClipboardData(CF_UNICODETEXT).context("failed to get clipboard data")?.0);
		
		let l_mem = GlobalLock(g_mem) as *const u16;
		if l_mem.is_null() {
			return Err(OsError::from_thread("failed to lock mem"));
		}
		
		unsafe extern "C" { fn wcslen(s: *const u16) -> usize; }
		let len = wcslen(l_mem);
		
		let s = if len != 0 {
			String::from_utf16_lossy(slice::from_raw_parts(l_mem, len))
		} else {
			String::default()
		}; 
		
		if let Err(err) = GlobalUnlock(g_mem) && err.code().0 != NO_ERROR.0 as _ {
			Err(OsError::new("failed to unlock mem", err))
		} else {
			Ok(s)
		}
	}
}

pub fn set_text(text: impl AsRef<str>) -> Result<(), OsError> {
	open_clipb().and_then(|_| set_clipb_text(text).and(close_clipb()))
}

fn set_clipb_text(text: impl AsRef<str>) -> Result<(), OsError> {
	let text = text.as_ref();
	
	unsafe {
		EmptyClipboard().context("failed to empty clipboard")?;
		
		let encoded: Vec<u16> = text
			.encode_utf16()
			.chain(once(0))
			.collect();
		
		let g_mem = {
			let h = GlobalAlloc(GMEM_MOVEABLE, encoded.len() * 2).context("failed to alloc")?;
			Owned::new(h) // we own this allocation until it's passed to SetClipboardData
		};
		
		let l_mem = GlobalLock(*g_mem) as *mut u16;
		if l_mem.is_null() {
			return Err(OsError::from_thread("failed to lock mem"));
		}
		
		ptr::copy_nonoverlapping(encoded.as_ptr(), l_mem, encoded.len());
		
		if let Err(err) = GlobalUnlock(*g_mem) && err.code().0 != NO_ERROR.0 as _ {
			return Err(OsError::new("failed to unlock mem", err));
		}
		
		_ = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(g_mem.0))).context("failed to set clipboard data")?;
		
		// The data is successfully set, so now OS owns the allocation.
		forget(g_mem);
		
		Ok(())
	}
}

fn open_clipb() -> Result<(), OsError> {
	// TODO: retry if failed
	unsafe { OpenClipboard(Some(super::hwnd())).context("failed to open clipboard") }
}

fn close_clipb() -> Result<(), OsError> {
	unsafe { CloseClipboard().context("failed to close clipboard") }
}

#[derive(Debug)]
pub enum GetTextError {
	FormatUnavailable,
	Other(OsError)
}

impl fmt::Display for GetTextError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			GetTextError::FormatUnavailable => write!(f, "FormatUnavailable"),
			GetTextError::Other(err) => err.fmt(f),
		}
	}
}

impl From<OsError> for GetTextError {
	fn from(value: OsError) -> Self {
		Self::Other(value)
	}
}

impl std::error::Error for GetTextError {}
