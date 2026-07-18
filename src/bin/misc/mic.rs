use std::{env, sync::{OnceLock, atomic::{AtomicBool, Ordering}}, process::Command, path::PathBuf};
use i44::misc::{Icon, audio::{self, Device, DeviceType, VolumeNotfEvent}, tray_icon::{IconEvent, TrayIcon}};
use i44::{App, common::error::{Error, OK}};
use super::sound;

static MIC: OnceLock<Device> = OnceLock::new();
static ICON: OnceLock<Icon> = OnceLock::new();
static MUTED: AtomicBool = AtomicBool::new(false);
static PATHS: OnceLock<Paths> = OnceLock::new();

#[derive(Debug)]
struct Paths {
	mic_unmuted: PathBuf,
	mic_muted: PathBuf,
}

pub fn init(app: &App) {
	const NAME: &str = "Microphone (FIFINE K670 Microphone)";
	
	let mut mic = audio::default_device(DeviceType::Capture).expect("failed to get mic");
	assert!(mic.name() == NAME, "wrong default mic");
	
	let mut dir = env::current_dir().expect("failed to get current dir");
	dir.push("media");
	
	let icon = app.icon_builder()
		.add("FIFINE K670", dir.join("greenMic.ico")).expect("failed to add green icon")
		.add("FIFINE K670 (muted)", dir.join("redMic.ico")).expect("failed to add red icon")
		.handler(icon_handler)
		.build();
	
	let muted = mic.get_mute().unwrap();
	icon.display(muted as _).expect("failed to display icon");
	
	MUTED.store(muted, Ordering::Relaxed);
	ICON.set(app.add_icon(icon)).expect("ICON should not be set");
	
	mic.on_volume_update(volume_handler).expect("failed to add volume handler");
	MIC.set(mic).expect("MIC should not be set");
	
	let paths = Paths {
		mic_unmuted: dir.join("Windows Hardware Insert.wav"),
		mic_muted: dir.join("Windows Hardware Fail.wav")
	};
	PATHS.set(paths).expect("Paths should not be set");
}

pub fn tgl_mute() -> Result<bool, Error> {
	get_mic().tgl_mute()
}

fn get_mic() -> &'static Device {
	MIC.get().expect("MIC should be initialized")
}

fn get_icon() -> &'static Icon {
	ICON.get().expect("ICON should be initialized")
}

fn get_paths() -> &'static Paths {
	PATHS.get().expect("Paths should be initialized")
}

fn icon_handler(_: &TrayIcon, event: IconEvent) -> Result<(), Error> {
	match event {
		IconEvent::LClick => tgl_mute().map(|_| ()),
		IconEvent::RClick => { Command::new("control").arg("mmsys.cpl,,1").spawn()?; OK },
		IconEvent::DClick => OK,
	}
}

fn volume_handler(event: &VolumeNotfEvent) -> Result<(), Error> {
	if MUTED.swap(event.muted, Ordering::Relaxed) != event.muted {
		get_icon().display(event.muted as _)?;
		let paths = get_paths();
		sound::play_vol(if event.muted { &paths.mic_muted } else { &paths.mic_unmuted }, 50)
	} else {
		OK
	}
}
