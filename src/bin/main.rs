mod misc;

use i44::App;
use misc::{hotkeys::AppExt, mode, kb::I44};

fn main() {
	let app = App::new()
		.add_hotkeys()
		.on_exit(|_| _ = I44::disable());
	
	mode::init();
	
	I44::enable().expect("failed to connect to kb");
	
	app.run();
}
