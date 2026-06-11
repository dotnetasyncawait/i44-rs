use crate::input::key_event::KeyEvent;
use crate::common::error::{Error, OK};
use windows::Win32::{
	Foundation::{POINT, RECT},
	UI::WindowsAndMessaging::{
		GA_ROOT, GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowRect, MoveWindow, SetCursorPos, WindowFromPoint}};


pub fn drag_win(key: KeyEvent) -> Result<(), Error> {
	let mut point = POINT::default();
	unsafe { GetCursorPos(&mut point)?; }
	
	let mut prev_mouse_x = point.x;
	let mut prev_mouse_y = point.y;
	
	let hwnd = unsafe { WindowFromPoint(point) };
	if hwnd.is_invalid() { 
		return Err(Error::Other("failed to get window from point".to_string()));
	}
	
	let hwnd = unsafe { GetAncestor(hwnd, GA_ROOT) };
	
	let mut rect = RECT::default();
	unsafe { GetWindowRect(hwnd, &mut rect)?; }
	
	let mut win_x = rect.left;
	let mut win_y = rect.top;
	let w = rect.right - rect.left;
	let h = rect.bottom - rect.top;
	
	loop {
		unsafe { GetCursorPos(&mut point)? };
		
		let mouse_x = point.x;
		let mouse_y = point.y;
		
		win_x += mouse_x - prev_mouse_x;
		win_y += mouse_y - prev_mouse_y;
		
		unsafe { MoveWindow(hwnd, win_x, win_y, w, h, true)?; }
		
		prev_mouse_x = mouse_x;
		prev_mouse_y = mouse_y;
		
		if key.is_up() {
			break;
		}
	}
	
	OK
}

pub fn center_cursor() -> Result<(), Error> {
	let mut rect = RECT::default();
	let hwnd = unsafe { GetForegroundWindow() };
	if hwnd.is_invalid() {
		return Err(Error::Other("hwnd == 0".to_string()));
	}
	unsafe { GetWindowRect(hwnd, &mut rect)?; }
	unsafe { SetCursorPos(rect.left + (rect.right - rect.left) / 2, rect.top  + (rect.bottom - rect.top) / 2)?; }
	OK
}