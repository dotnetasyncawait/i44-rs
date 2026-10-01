fn main() {
	embed_manifest();
}

fn embed_manifest() {
	use std::env;
	
	let is_uia = env::var_os("CARGO_FEATURE_UIA").is_some();
	let is_win_os = env::var_os("CARGO_CFG_WINDOWS").is_some();
	let is_msvc = Ok("msvc") == env::var("CARGO_CFG_TARGET_ENV").as_deref();
	
	if is_uia && is_win_os && is_msvc {
		static MANIFEST_FILE: &str = "app.manifest";
	
		let mut manifest = env::current_dir().unwrap();
		manifest.push(MANIFEST_FILE);
		
		println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
		println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.to_str().unwrap());
		println!("cargo:rustc-link-arg-bins=/MANIFESTUAC:NO");
	
		// Turn linker warnings into errors.
		// println!("cargo:rustc-link-arg-bins=/WX");
	}
}