mod misc;
mod system;

use i44::{apps::explorer};
use misc::{hotkeys::AppExt, mode, kb::{self, hid_msgs::HID_DEFAULT}, mic, sound};
use windows::Win32::{
	Foundation::{HWND, LPARAM, WPARAM},
	UI::WindowsAndMessaging::{PBT_APMRESUMEAUTOMATIC, WM_POWERBROADCAST}};

fn main() {
	let app = i44::new()
		.add_hotkeys()
		.on_message(WM_POWERBROADCAST, default_kb)
		.on_exit(|| { _ = kb::disable(); false });
	
	sound::init();
	mode::init();
	mic::init();
	explorer::init();
	
	kb::enable().expect("failed to connect to kb");
	
	app.run();
}

// TODO: return Result<Option<isize>, Error>
fn default_kb(_: HWND, _: u32, wparam: WPARAM, _: LPARAM) -> Option<isize> {
	if wparam.0 as u32 == PBT_APMRESUMEAUTOMATIC {
		let mut kb = kb::new_device();
		kb.open()
			.and_then(|_| kb.write(&[]))
			.and_then(|_| kb.write(&[HID_DEFAULT]))
			.expect("failed to default kb");
	}
	None
}
