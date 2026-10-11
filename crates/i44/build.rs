use std::{env, fs, io, path::{Path, PathBuf}, process::Command};

fn main() {
	copy_media_dir();
	build_and_embed_rc();
}

fn copy_media_dir() {
	let dst = PathBuf::from(env::var("OUT_DIR").unwrap())
		.ancestors()
		.nth(3)
		.unwrap()
		.join("media");
	
	let src = "../../media";
	println!("cargo::rerun-if-changed={src}");
	
	if let Err(err) = fs::remove_dir_all(&dst) && err.kind() != io::ErrorKind::NotFound {
		panic!("failed to remove dst dir '{}': {err}", dst.display());
	}
	
	copy_dir(src, dst);
}

fn copy_dir(src_dir: impl Into<PathBuf>, dst_dir: impl Into<PathBuf>) {
	let mut dirs = vec![(src_dir.into(), dst_dir.into())];
	
	while let Some((src_dir, dst_dir)) = dirs.pop() {
		if let Err(err) = fs::create_dir(&dst_dir) && err.kind() != io::ErrorKind::AlreadyExists {
			panic!("failed to create dst dir '{}': {err}", dst_dir.display());
		}
		
		for item in fs::read_dir(src_dir).unwrap().map(|d| d.unwrap()) {
			let src = item.path();
			let dst = dst_dir.join(item.file_name());
			
			if item.file_type().unwrap().is_dir() {
				dirs.push((src, dst));
			} else {
				fs::copy(src, dst).unwrap();
			}
		}
	}
}

fn build_and_embed_rc() {
	assert!(env::var_os("CARGO_CFG_WINDOWS").is_some());
	assert_eq!(Ok("msvc"), env::var("CARGO_CFG_TARGET_ENV").as_deref());
	
	let out = PathBuf::from(env::var("OUT_DIR").unwrap());
	let manifest_path = out.join("app.manifest");
	
	build_manifest(&manifest_path);
	build_rc(&manifest_path, &out.join("app.rc"));
	
	println!("cargo::rustc-link-arg-bins={}", out.join("app.res").display());
}

fn build_manifest(manifest_path: &Path) {
	let is_uia = env::var_os("CARGO_FEATURE_UIA").is_some();
	
	let manifest = format!(r#"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
	<compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
		<application>
			<!-- Windows 10 and Windows 11 -->
			<supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}" />
		</application>
	</compatibility>
	
	<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
		<security>
			<requestedPrivileges>
				<requestedExecutionLevel level="asInvoker" uiAccess="{is_uia}"/>
			</requestedPrivileges>
		</security>
	</trustInfo>
</assembly>
"#);
	
	fs::write(manifest_path, manifest).unwrap();
}

fn build_rc(manifest_path: &Path, rc_path: &Path) {
	let rc = format!(r#"
1 ICON "../../media/default.ico"
1 24 "{}"
"#, manifest_path.display().to_string().replace('\\', "/"));
	
	fs::write(rc_path, rc).unwrap();
	
	// Note: this only works if 'rc.exe' is reachable via PATH env variable,
	// which is "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64".
	// TODO: use fully qualified path
	Command::new("rc")
		.arg(rc_path)
		.status()
		.unwrap();
}
