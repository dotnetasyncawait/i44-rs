mod misc;

use i44::App;
use misc::{hotkeys::AppExt, mode, kb::{I44, hid_msgs::HID_DEFAULT}};
use windows::Win32::{
	Foundation::{HWND, LPARAM, WPARAM},
	UI::WindowsAndMessaging::{PBT_APMRESUMEAUTOMATIC, WM_POWERBROADCAST}};

fn main() {
	let app = App::new()
		.add_hotkeys()
		.on_message(WM_POWERBROADCAST, default_kb)
		.on_exit(|_| _ = I44::disable());
	
	mode::init();
	
	I44::enable().expect("failed to connect to kb");
	
	app.run();
}

fn default_kb(_: HWND, _: u32, wparam: WPARAM, _: LPARAM) -> isize {
	if wparam.0 as u32 == PBT_APMRESUMEAUTOMATIC {
		let mut kb = I44::new_device();
		
		kb.open()
			.and_then(|_| kb.write(&[]))
			.and_then(|_| kb.write(&[HID_DEFAULT]))
			.expect("failed to default kb");
	}
	
	0
}