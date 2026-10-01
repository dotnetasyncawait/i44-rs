use crate::common::error::{OsError, Win32ErrResExt, tagged_error};
use crate::input::key_event::KeyEvent;
use std::path::Path;
use windows::core::PWSTR;
use windows::Win32::{
	System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
	Foundation::{GetLastError, SetLastError, WIN32_ERROR, HWND, POINT, RECT},
	UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
		GetClassNameW, GA_ROOT, GetAncestor, GetCursorPos, GetWindowRect, MoveWindow, SetCursorPos, WindowFromPoint,
		IsZoomed, IsIconic, SetForegroundWindow}};

tagged_error!(
	Error,
	ErrorKind { NotFound, InvalidHwnd, Os }
);

impl Error {
	pub unsafe fn into_os_unchecked(self) -> OsError {
		unsafe { self.into_extra_unchecked() }
	}
}

impl From<OsError> for Error {
	fn from(value: OsError) -> Self {
		Self::new_custom(ErrorKind::Os, value)
	}
}

tagged_error!(
	WinDragError,
	WinDragErrorKind { NotFound, Maximized, Os }
);

impl From<OsError> for WinDragError {
	fn from(value: OsError) -> Self {
		Self::new_custom(WinDragErrorKind::Os, value)
	}
}

/// Returns name of a foreground window.
/// # Errors
/// - [ErrorKind::NotFound]: no foreground window
/// - [ErrorKind::Os]: system error
pub fn name() -> Result<String, Error> {
	let hwnd = unsafe { GetForegroundWindow() };
	inner(hwnd, ErrorKind::NotFound, || title_inner(hwnd))
}

/// Returns name of a specified window.
/// # Errors
/// - [ErrorKind::InvalidHwnd]: argument `hwnd` is invalid
/// - [ErrorKind::Os]: system error
pub fn name_of(hwnd: HWND) -> Result<String, Error> {
	inner(hwnd, ErrorKind::InvalidHwnd, || name_inner(hwnd))
}

fn name_inner(hwnd: HWND) -> Result<String, OsError> {
	debug_assert!(!hwnd.is_invalid());
	
	let mut process_id: u32 = 0;
	let ret = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
	if ret == 0 {
		return Err(OsError::from_thread("failed to get window thread process id"));
	}
	
	let handle = unsafe { OpenProcess(
		PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).context("failed to open process")? };
	
	let mut arr = [0u16; 1024];
	let mut buff_size = arr.len() as u32;
	
	unsafe { QueryFullProcessImageNameW(
		handle, PROCESS_NAME_WIN32, PWSTR(arr.as_mut_ptr()), &mut buff_size).context("failed to query process image name")?;
	}
	
	let full_path = String::from_utf16_lossy(&arr[..buff_size as _]);
	
	let name_os = Path::new(&full_path)
		.file_prefix()
		.expect("path should contain file prefix");
	
	// SAFETY: 'name_os' is originated from 'full_path', which is valid UTF-8.
	let name_str = unsafe { str::from_utf8_unchecked(name_os.as_encoded_bytes()) };
	
	Ok(String::from(name_str))
}

/// Returns title of a foreground window.
/// # Errors
/// - [ErrorKind::NotFound]: no foreground window
/// - [ErrorKind::Os]: system error
pub fn title() -> Result<String, Error> {
	let hwnd = unsafe { GetForegroundWindow() };
	inner(hwnd, ErrorKind::NotFound, || title_inner(hwnd))
}

/// Returns title of a specified window.
/// # Errors
/// - [ErrorKind::InvalidHwnd]: argument `hwnd` is invalid
/// - [ErrorKind::Os]: system error
pub fn title_of(hwnd: HWND) -> Result<String, Error> {
	inner(hwnd, ErrorKind::InvalidHwnd, || title_inner(hwnd))
}

fn title_inner(hwnd: HWND) -> Result<String, OsError> {
	debug_assert!(!hwnd.is_invalid());
	
	unsafe { SetLastError(WIN32_ERROR(0)); }
	let len = unsafe { GetWindowTextLengthW(hwnd) as usize };
	
	if len != 0 {
		// TODO: replace with boxed slice
		let mut buff = vec![0u16; len + 1]; // + NULL
		let n = unsafe { GetWindowTextW(hwnd, &mut buff) as usize };
		Ok(String::from_utf16_lossy(&buff[..n]))
	} else {
		let err_code = unsafe { GetLastError() };
		if err_code.is_err() {
			Err(OsError::from_win32("failed to get window text length", err_code))
		} else {
			Ok(String::default())
		}
	}
}

/// Returns class name of a foreground window.
/// # Errors
/// - [ErrorKind::NotFound]: no foreground window
/// - [ErrorKind::Os]: system error
pub fn class() -> Result<String, Error> {
	let hwnd = unsafe { GetForegroundWindow() };
	inner(hwnd, ErrorKind::NotFound, || class_inner(hwnd))
}

/// Returns class name of a specified window.
/// # Errors
/// - [ErrorKind::InvalidHwnd]: argument `hwnd` is invalid
/// - [ErrorKind::Os]: system error
pub fn class_of(hwnd: HWND) -> Result<String, Error> {
	inner(hwnd, ErrorKind::InvalidHwnd, || class_inner(hwnd))
}

fn class_inner(hwnd: HWND) -> Result<String, OsError> {
	debug_assert!(!hwnd.is_invalid());
	
	let mut buff = [0u16; 256];
	let n = unsafe { GetClassNameW(hwnd, &mut buff) as usize };
	
	if n == 0 {
		Err(OsError::from_thread("failed to get class name"))
	} else {
		Ok(String::from_utf16_lossy(&buff[..n]))
	}
}

pub fn is_maximized(hwnd: HWND) -> bool {
	!hwnd.is_invalid() && unsafe { IsZoomed(hwnd).as_bool() }
}

pub fn is_minimized(hwnd: HWND) -> bool {
	!hwnd.is_invalid() && unsafe { IsIconic(hwnd).as_bool() }
}

fn inner(hwnd: HWND, if_invalid: ErrorKind, f: impl FnOnce() -> Result<String, OsError>) -> Result<String, Error> {
	if hwnd.is_invalid() {
		Err(Error::new_simple(if_invalid))
	} else {
		Ok(f()?)
	}
}

/// Drags a window (that's currently under the cursor) while the specified key is down.
/// # Errors
/// - [WinDragErrorKind::NotFound]: no window is under the cursor
/// - [WinDragErrorKind::Maximized]: target window is maximized; therefore, it won't be moved
/// - [WinDragErrorKind::Os]: system error
pub fn drag(key: KeyEvent) -> Result<(), WinDragError> {
	let mut point = POINT::default();
	unsafe { GetCursorPos(&mut point).context("failed to get cursor pos")?; }
	
	let mut prev_mouse_x = point.x;
	let mut prev_mouse_y = point.y;
	
	let hwnd = unsafe { WindowFromPoint(point) };
	if hwnd.is_invalid() {
		return Err(WinDragError::new_simple(WinDragErrorKind::NotFound));
	}
	let hwnd = unsafe { GetAncestor(hwnd, GA_ROOT) };
	
	if is_maximized(hwnd) {
		return Err(WinDragError::new_simple(WinDragErrorKind::Maximized))
	}
	
	if unsafe { !SetForegroundWindow(hwnd).as_bool() } {
		let msg = format!("failed to set foreground window ({hwnd:?})");
		if cfg!(debug_assertions) {
			println!("{msg}");
			return Ok(());
		} else {
			panic!("{msg}");
		}
	}
	
	let mut rect = RECT::default();
	unsafe { GetWindowRect(hwnd, &mut rect).context("failed to get window rect")?; }
	
	let mut win_x = rect.left;
	let mut win_y = rect.top;
	let w = rect.right - rect.left;
	let h = rect.bottom - rect.top;
	
	loop {
		unsafe { GetCursorPos(&mut point).context("failed to get cursor pos")?; }
		
		let mouse_x = point.x;
		let mouse_y = point.y;
		
		win_x += mouse_x - prev_mouse_x;
		win_y += mouse_y - prev_mouse_y;
		
		unsafe { MoveWindow(hwnd, win_x, win_y, w, h, true).context("failed to move window")?; }
		
		prev_mouse_x = mouse_x;
		prev_mouse_y = mouse_y;
		
		if key.is_up() {
			break;
		}
	}
	
	Ok(())
}

/// Moves cursor to the center of a foreground window.
/// # Errros
/// - [ErrorKind::NotFound]: no foreground window
/// - [ErrorKind::Os]: system error
pub fn center_cursor() -> Result<(), Error> {
	let mut rect = RECT::default();
	let hwnd = unsafe { GetForegroundWindow() };
	
	if hwnd.is_invalid() {
		Err(Error::new_simple(ErrorKind::NotFound))
	} else {
		unsafe {
			GetWindowRect(hwnd, &mut rect).context("failed to get window rect")?;
			SetCursorPos(
				rect.left + (rect.right - rect.left) / 2,
				rect.top  + (rect.bottom - rect.top) / 2).context("failed to set cursor pos")?;
		}
		Ok(())
	}
}
