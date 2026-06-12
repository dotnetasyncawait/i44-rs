mod hotkeys;

use i44::common::kb::I44;
use i44::App;
use hotkeys::AppExt;

fn main() {
	let app = App::new()
		.add_hotkeys()
		.on_exit(|_| { let _ = I44::disable(); });
	
	I44::enable().expect("failed to connect to kb");
	
	app.run();
}
