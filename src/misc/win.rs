use crate::common::error::{Error, Win32Error};
use std::path::Path;
use windows::Win32::{UI::WindowsAndMessaging::GetClassNameW, Foundation::HWND};
use windows::core::PWSTR;
use windows::Win32::{
	System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
	UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId}};


pub fn name() -> Result<String, Error> {
	let hwnd = unsafe { GetForegroundWindow() };
	if hwnd.is_invalid() {
		return Err(Error::Other(String::from("hwnd is 0")));
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
	let hwnd = unsafe { GetForegroundWindow() };
	if hwnd.is_invalid() {
		return Err(Error::Other("hwnd == 0".to_string()))
	}
	
	let len = unsafe { GetWindowTextLengthW(hwnd) as usize };
	let mut buff = vec![0u16; len + 1]; // + NULL
	let copied = unsafe { GetWindowTextW(hwnd, &mut buff) } as usize ;
	
	Ok(String::from_utf16(&buff[..copied])?)
}

pub fn class_of(hwnd: HWND) -> String {
	let mut buff = [0u16; 256];
	let n = unsafe { GetClassNameW(hwnd, &mut buff) as usize };
	String::from_utf16(&buff[..n]).expect("class name must be valid UTF-16")
}
