mod misc;
mod system;

use rhk::{common::error::Error};
use misc::{hotkeys::AppExt, hotstrs::AppExt as _, mode, mic, sound, kb::{self, hid_msgs::HID_DEFAULT}};
use windows::Win32::{
	Foundation::{HWND, LPARAM, WPARAM},
	UI::WindowsAndMessaging::{PBT_APMRESUMEAUTOMATIC, WM_POWERBROADCAST}};

fn main() {
	let app = rhk::new()
		.add_hotkeys()
		.add_hotstrs()
		.on_message(WM_POWERBROADCAST, default_kb)
		.on_exit(|| { _ = kb::disable(); false });
	
	sound::init();
	mode::init();
	mic::init();
	kb::init();
	
	app.run();
}

fn default_kb(_: HWND, _: u32, wparam: WPARAM, _: LPARAM) -> Result<Option<isize>, Error> {
	if wparam.0 as u32 == PBT_APMRESUMEAUTOMATIC {
		let mut kb = kb::new_device();
		kb.open()
			.and_then(|_| kb.write(&[]))
			.and_then(|_| kb.write(&[HID_DEFAULT]))?;
	}
	Ok(None)
}
