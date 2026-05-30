fn main() {
	i44::App::new()
		.add_hotkeys()
		.run();
}

trait AppExt {
	fn add_hotkeys(self) -> Self;
}

impl AppExt for i44::App {
	fn add_hotkeys(self) -> Self {
		self
	}
}