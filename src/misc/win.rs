use crate::common::error::{Error, Win32Error};
use std::path::Path;
use windows::core::PWSTR;
use windows::Win32::{
	System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
	Foundation::{GetLastError, SetLastError, WIN32_ERROR, HWND},
	UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
		GetClassNameW}};

pub fn name() -> Result<String, Error> {
	name_of(unsafe { GetForegroundWindow() })
}

pub fn name_of(hwnd: HWND) -> Result<String, Error> {
	if hwnd.is_invalid() {
		return Err(Error::other("invalid HWND"));
	}
	
	let mut process_id: u32 = 0;
	let ret = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
	if ret == 0 {
		return Err(Win32Error::from_thread().into());
	}
	
	let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id)? };
	
	let mut arr = [0u16; 1024];
	let mut buff_size = arr.len() as u32;
	
	unsafe { QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(arr.as_mut_ptr()), &mut buff_size)? }
	
	let full_path = String::from_utf16(&arr[..(buff_size as _)])?;
	
	let name = Path::new(&full_path)
		.file_prefix()
		.ok_or_else(|| Error::Other(format!("failed to get file prefix: '{full_path}'")))?
		.to_str()
		.ok_or_else(|| Error::Other(format!("failed to convert into str: '{full_path}'")))?;
	
	Ok(String::from(name))
}

pub fn title() -> Result<String, Error> {
	title_of(unsafe { GetForegroundWindow() })
}

pub fn title_of(hwnd: HWND) -> Result<String, Error> {
	if hwnd.is_invalid() {
		return Err(Error::other("invalid HWND"));
	}
	
	unsafe { SetLastError(WIN32_ERROR(0)); }
	let len = unsafe { GetWindowTextLengthW(hwnd) as usize };
	
	if len != 0 {
		let mut buff = vec![0u16; len + 1]; // + NULL
		let n = unsafe { GetWindowTextW(hwnd, &mut buff) as usize };
		Ok(String::from_utf16_lossy(&buff[..n]))
	} else {
		let err_code = unsafe { GetLastError() };
		if err_code.is_err() {
			Err(Win32Error::from(err_code).into())
		} else {
			Ok(String::default())
		}
	}
}

pub fn class() -> Result<String, Error> {
	class_of(unsafe { GetForegroundWindow() })
}

pub fn class_of(hwnd: HWND) -> Result<String, Error> {
	if hwnd.is_invalid() {
		Err(Error::other("invalid HWND"))
	} else {
		let mut buff = [0u16; 256];
		let n = unsafe { GetClassNameW(hwnd, &mut buff) as usize };
		if n == 0 {
			Err(Win32Error::from_thread().into())
		} else {
			Ok(String::from_utf16_lossy(&buff[..n]))
		}
	}
}
