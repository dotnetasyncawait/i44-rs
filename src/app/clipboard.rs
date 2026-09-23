use core::fmt;
use std::{iter::once, mem::forget, ptr, slice};
use crate::common::error::{OsError, Win32ErrResExt};
use windows_core::{HRESULT, Owned};
use windows::Win32::{
	Foundation::{GetLastError, HANDLE, HGLOBAL, NO_ERROR},
	System::Ole::{self, CLIPBOARD_FORMAT},
	System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
	System::DataExchange::{
		CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
		SetClipboardData}};

const CF_UNICODETEXT: u32 = Ole::CF_UNICODETEXT.0 as u32;
const HR_NO_ERROR: HRESULT = NO_ERROR.to_hresult();

pub fn get_text() -> Result<String, GetTextError> {
	if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT).is_ok() } {
		open_clipb()?;
		let res = get_text_inner();
		let close_res = close_clipb();
		Ok(res.and_then(|s| close_res.map(|_| s))?)
	} else {
		Err(GetTextError::FormatUnavailable)
	}
}

fn get_text_inner() -> Result<String, OsError> {
	unsafe {
		let g_mem = HGLOBAL(GetClipboardData(CF_UNICODETEXT).context("failed to get clipboard data")?.0);
		
		let l_mem = GlobalLock(g_mem) as *const u16;
		if l_mem.is_null() {
			return Err(OsError::from_thread("failed to lock mem"));
		}
		
		// TODO: use GlobalSize()?
		unsafe extern "C" { fn wcslen(s: *const u16) -> usize; }
		let len = wcslen(l_mem);
		
		let s = if len != 0 {
			String::from_utf16_lossy(slice::from_raw_parts(l_mem, len))
		} else {
			String::default()
		}; 
		
		if let Err(err) = GlobalUnlock(g_mem) && err.code() != HR_NO_ERROR {
			Err(OsError::new("failed to unlock mem", err))
		} else {
			Ok(s)
		}
	}
}

pub fn set_text(text: impl AsRef<str>) -> Result<(), OsError> {
	open_clipb().and_then(|_| set_text_inner(text).and(close_clipb()))
}

fn set_text_inner(text: impl AsRef<str>) -> Result<(), OsError> {
	let text = text.as_ref();
	
	unsafe {
		EmptyClipboard().context("failed to empty clipboard")?;
		if text.is_empty() {
			return Ok(());
		}
		
		let encoded: Vec<u16> = text
			.encode_utf16()
			.chain(once(0))
			.collect();
		
		let g_mem = {
			let h = GlobalAlloc(GMEM_MOVEABLE, encoded.len() * 2).context("failed to global alloc")?;
			Owned::new(h) // we own this allocation until it's passed to SetClipboardData()
		};
		
		let l_mem = GlobalLock(*g_mem) as *mut u16;
		if l_mem.is_null() {
			return Err(OsError::from_thread("failed to lock mem"));
		}
		
		ptr::copy_nonoverlapping(encoded.as_ptr(), l_mem, encoded.len());
		
		if let Err(err) = GlobalUnlock(*g_mem) && err.code() != HR_NO_ERROR {
			return Err(OsError::new("failed to unlock mem", err));
		}
		
		_ = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(g_mem.0))).context("failed to set clipboard data")?;
		
		// The data is successfully set, so now OS owns the allocation.
		forget(g_mem);
		
		Ok(())
	}
}

pub fn get_raw() -> Result<RawData, OsError> {
	open_clipb()?;
	let raw_res = get_raw_inner();
	let close_res = close_clipb();
	raw_res.and_then(|data| close_res.map(|_| data))
}

fn get_raw_inner() -> Result<RawData, OsError> {
	// Synthesized conversions provided by OS:
	// CF_BITMAP -> [CF_DIB, CF_DIBV5]
	// CF_DIB    -> [CF_BITMAP, CF_PALETTE, CF_DIBV5]
	// CF_DIBV5  -> [CF_BITMAP, CF_DIB, CF_PALETTE]
	// 
	// CF_ENHMETAFILE  -> CF_METAFILEPICT
	// CF_METAFILEPICT -> CF_ENHMETAFILE
	//
	// CF_OEMTEXT     -> [CF_TEXT, CF_UNICODETEXT]
	// CF_TEXT        -> [CF_OEMTEXT, CF_UNICODETEXT]
	// CF_UNICODETEXT -> [CF_OEM_TEX, CF_TEXT]
	// https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#synthesized-clipboard-formats
	
	let mut total_size = 0;
	let mut items = Vec::<(u32, u32, HGLOBAL)>::new();
	let mut state = 0u8;
	
	let mut next = 0u32;
	while let fmt = unsafe { EnumClipboardFormats(next) } && fmt != 0 {
		next = fmt;
		
		const BIT_DIB:         u8 = 0x01;
		const BIT_DIBV5:       u8 = 0x02;
		const BIT_TEXT:        u8 = 0x10;
		const BIT_OEMTEXT:     u8 = 0x20;
		const BIT_UNICODETEXT: u8 = 0x40;
		
		match CLIPBOARD_FORMAT(fmt as u16) {
			// GlobalSize() fails for CF_BITMAP, so we skip it and use CF_DIB or CF_DIBV5 instead.
			// According to AutoHotkey's ClipboardAll() implementation, CF_ENHMETAFILE and CF_DSPENHMETAFILE
			// fail as well, so we skip them too.
			// https://github.com/AutoHotkey/AutoHotkey/blob/v2.0/source/var.cpp#L342
			Ole::CF_BITMAP | Ole::CF_ENHMETAFILE | Ole::CF_DSPENHMETAFILE => continue,
			
			Ole::CF_DIB => if state & BIT_DIBV5 != 0 { continue; } else { state |= BIT_DIB; }
			Ole::CF_DIBV5 => if state & BIT_DIB != 0 { continue; } else { state |= BIT_DIBV5; }
			Ole::CF_PALETTE => if state & (BIT_DIB | BIT_DIBV5) != 0 { continue; }
			Ole::CF_TEXT => if state & (BIT_OEMTEXT | BIT_UNICODETEXT) != 0 { continue; } else { state |= BIT_TEXT; }
			Ole::CF_OEMTEXT => if state & (BIT_TEXT | BIT_UNICODETEXT) != 0 { continue; } else { state |= BIT_OEMTEXT; }
			Ole::CF_UNICODETEXT => if state & (BIT_TEXT | BIT_OEMTEXT) != 0 { continue; } else { state |= BIT_UNICODETEXT; }
			_ => {}
		}
		
		let g_mem = HGLOBAL(unsafe { GetClipboardData(fmt).context("failed to get clipboard data")?.0 });
		
		let size = unsafe { GlobalSize(g_mem) };
		if size == 0 {
			continue;
		}
		
		total_size += 4 + 4 + size; // fmt + size + data
		items.push((fmt, size as u32, g_mem));
	}
	
	if let err_code = unsafe { GetLastError() } && err_code.is_err() {
		return Err(OsError::from_win32("failed to enumerate clipboard formats", err_code));
	}
	
	if total_size == 0 {
		return Ok(RawData { data: unsafe { Box::new_uninit_slice(0).assume_init() }});
	}
	
	total_size += 4; // NULL
	
	let mut data = Box::<[u8]>::new_uninit_slice(total_size);
	let ptr = data.as_mut_ptr() as *mut u8;
	let mut offset = 0;
	
	for (fmt, size, g_mem) in items {
		unsafe { (ptr.offset(offset) as *mut u32).write(fmt); }
		offset += 4;
		unsafe { (ptr.offset(offset) as *mut u32).write(size); }
		offset += 4;
		
		let l_mem = unsafe { GlobalLock(g_mem) as *const u8 };
		if l_mem.is_null() {
			return Err(OsError::from_thread("failed to lock mem"));
		}
		unsafe { ptr::copy_nonoverlapping(l_mem, ptr.offset(offset), size as usize) };
		offset += size as isize;
		
		if let Err(err) = unsafe { GlobalUnlock(g_mem) } && err.code() != HR_NO_ERROR {
			return Err(OsError::new("failed to unlock mem", err));
		}
	}
	
	unsafe { (ptr.offset((data.len()-4) as isize) as *mut u32).write(0) };
	Ok(RawData { data: unsafe { data.assume_init() } })
}

pub fn set_raw(data: RawData) -> Result<(), OsError> {
	open_clipb().and_then(|_| set_raw_inner(data).and(close_clipb()))
}

fn set_raw_inner(raw: RawData) -> Result<(), OsError> {
	unsafe { EmptyClipboard().context("failed to empty clipboard")?; }
	if raw.is_empty() {
		return Ok(());
	}
	
	let slice = raw.data();
	let mut offset = 0;
	
	loop {
		let fmt = u32::from_le_bytes(slice[offset..offset+4].try_into().unwrap());
		offset += 4;
		if fmt == 0 {
			break;
		}
		
		let size = u32::from_le_bytes(slice[offset..offset+4].try_into().unwrap()) as usize;
		offset += 4;
		
		let data = &slice[offset..offset+size];
		offset += size;
		
		let g_mem = unsafe {
			let h = GlobalAlloc(GMEM_MOVEABLE, size).context("failed to global alloc")?;
			Owned::new(h) // we own this allocation until it's passed to SetClipboardData()
		};
		
		let l_mem = unsafe { GlobalLock(*g_mem) as *mut u8 };
		if l_mem.is_null() {
			return Err(OsError::from_thread("failed to lock mem"));
		}
		
		unsafe { ptr::copy_nonoverlapping(data.as_ptr(), l_mem, size); }
		
		if let Err(err) = unsafe { GlobalUnlock(*g_mem) } && err.code() != HR_NO_ERROR {
			return Err(OsError::new("failed to unlock mem", err));
		}
		
		_ = unsafe { SetClipboardData(fmt, Some(HANDLE(g_mem.0)))
			.with_context(|| format!("failed to set clipboard data; fmt: 0x{fmt:X}"))? };
		
		// The data is successfully set, so now OS owns the allocation.
		forget(g_mem);
	}
	
	Ok(())
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

pub struct RawData {
	data: Box<[u8]>
}

impl RawData {
	pub fn is_empty(&self) -> bool { self.data.len() == 0 }
	pub fn size(&self) -> usize { self.data.len() }
	pub fn data(&self) -> &[u8] { &self.data }
}
