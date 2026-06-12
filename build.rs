fn main() {
	embed_manifest();
}

fn embed_manifest() {
	use std::env;
	
	let embed = cfg!(feature = "uia") || Ok("release") == env::var("PROFILE").as_deref();
	let is_win_os = env::var("CARGO_CFG_WINDOWS").is_ok();
	let is_msvc = Ok("msvc") == env::var("CARGO_CFG_TARGET_ENV").as_deref();
	
	if embed && is_win_os && is_msvc {
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