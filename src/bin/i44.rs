mod misc;
mod system;

use i44::{apps::explorer};
use misc::{hotkeys::AppExt, mode, kb::{I44, hid_msgs::HID_DEFAULT}, mic, sound};
use windows::Win32::{
	Foundation::{HWND, LPARAM, WPARAM},
	System::Com::{COINIT_MULTITHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx},
	UI::WindowsAndMessaging::{PBT_APMRESUMEAUTOMATIC, WM_POWERBROADCAST}};

fn main() {
	set_panic_hook();
	unsafe { CoInitializeEx(None, COINIT_MULTITHREADED | COINIT_DISABLE_OLE1DDE).unwrap(); }
	
	let app = i44::new()
		.add_hotkeys()
		.on_message(WM_POWERBROADCAST, default_kb)
		.on_exit(|| { _ = I44::disable(); false });
	
	sound::init();
	mode::init();
	mic::init();
	explorer::init();
	
	I44::enable().expect("failed to connect to kb");
	
	app.run();
}

fn default_kb(_: HWND, _: u32, wparam: WPARAM, _: LPARAM) -> Option<isize> {
	if wparam.0 as u32 == PBT_APMRESUMEAUTOMATIC {
		let mut kb = I44::new_device();
		kb.open()
			.and_then(|_| kb.write(&[]))
			.and_then(|_| kb.write(&[HID_DEFAULT]))
			.expect("failed to default kb");
	}
	None
}

fn set_panic_hook() {
	use windows::{core::{PCSTR, s}, Win32::UI::WindowsAndMessaging::{MessageBoxA, MB_ICONERROR}};
	
	std::panic::set_hook(Box::new(|info| {
		let loc = info.location().unwrap();
		
		let text = format!(
			"panic at {}:{}:{}: '{}'\0",
			loc.file(), loc.line(), loc.column(), info.payload_as_str().unwrap_or_default());
		
		unsafe { MessageBoxA(None, PCSTR(text.as_ptr()), s!("Error"), MB_ICONERROR); }
	}));
}