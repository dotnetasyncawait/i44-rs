use crate::misc::error::{Error, Win32Error};
use std::path::Path;
use windows::{Win32::{System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
	QueryFullProcessImageNameW}, UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId}}, core::PWSTR};

pub fn get_name() -> Result<String, Error> {
	let hwnd = unsafe { GetForegroundWindow() };
	if hwnd.is_invalid() {
		return Err(Error::Other(String::from("hwnd is 0")));
	}
	
	let mut process_id: u32 = 0;
	let ret = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
	if ret == 0 {
		return Err(Error::Win32(Win32Error::from_thread()));
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
		.ok_or_else(|| Error::Other("failed to convert into str".to_string()))?;
	
	Ok(String::from(name))
}