use std::{env, sync::{OnceLock, atomic::{AtomicBool, AtomicU8, Ordering}}, process::Command, path::PathBuf};
use rhk::misc::audio::{self, Device, DeviceType, VolumeNotfEvent};
use rhk::{common::error::{Error, OsError, OK}, tray_icon::{Icon, TrayIcon, IconEvent}};
use super::sound;

static MIC: OnceLock<Device> = OnceLock::new();
static ICON: OnceLock<Icon> = OnceLock::new();
static PATHS: OnceLock<[PathBuf; 2]> = OnceLock::new();

static MUTED: AtomicBool = AtomicBool::new(false);
static VOLUME: AtomicU8 = AtomicU8::new(0);

static MIC_DISP_NAME: &str = "FIFINE K670";

#[derive(Debug)]
struct Paths {
	mic_unmuted: PathBuf,
	mic_muted: PathBuf,
}

pub fn init() {
	const NAME: &str = "Microphone (FIFINE K670 Microphone)";
	
	let mut mic = audio::default_device(DeviceType::Capture).expect("failed to get mic");
	assert!(mic.name() == NAME, "wrong default mic");
	
	let vol = mic.get_volume().expect("failed to get mic volume") as u8;
	
	let mut dir = env::current_dir().expect("failed to get current dir");
	dir.push("media");
	
	let icon = rhk::icon_builder()
		.add(dir.join("greenMic.ico"), format!("{MIC_DISP_NAME} ({vol}%)")).expect("failed to add green icon")
		.add(dir.join("redMic.ico"),   format!("{MIC_DISP_NAME} ({vol}%; muted)")).expect("failed to add red icon")
		.handler(icon_handler)
		.build()
		.expect("failed to build icon");
	
	let muted = mic.get_mute().unwrap();
	icon.display(muted as _).expect("failed to display icon");
	
	MUTED.store(muted, Ordering::Relaxed);
	ICON.set(rhk::add_icon(icon)).expect("ICON should not be set");
	
	mic.on_volume_update(volume_handler).expect("failed to add volume handler");
	MIC.set(mic).expect("MIC should not be set");
	
	let paths = [ dir.join("Windows Hardware Insert.wav"), dir.join("Windows Hardware Fail.wav") ];
	PATHS.set(paths).expect("Paths should not be set");
}

pub fn tgl_mute() -> Result<bool, OsError> {
	get_mic().tgl_mute()
}

fn get_mic() -> &'static Device {
	MIC.get().expect("MIC should be initialized")
}

fn get_icon() -> &'static Icon {
	ICON.get().expect("ICON should be initialized")
}

fn get_paths() -> &'static [PathBuf; 2] {
	PATHS.get().expect("Paths should be initialized")
}

fn icon_handler(_: &TrayIcon, event: IconEvent) -> Result<(), Error> {
	match event {
		IconEvent::LClick => Ok(_ = tgl_mute()?),
		IconEvent::RClick => { Command::new("control").arg("mmsys.cpl,,1").spawn()?; OK },
		IconEvent::DClick => OK,
	}
}

fn volume_handler(event: &VolumeNotfEvent) -> Result<(), Error> {
	let icon = get_icon();
	
	let vol = event.vol;
	let muted = event.muted;
	
	if VOLUME.swap(vol, Ordering::Relaxed) != vol {
		icon.update_tip(0, format!("{MIC_DISP_NAME} ({vol}%)"))?;
		icon.update_tip(1, format!("{MIC_DISP_NAME} ({vol}%; muted)"))?;
	}
	
	if MUTED.swap(muted, Ordering::Relaxed) != muted {
		let index = muted as usize;
		icon.display(index)?;
		sound::play_vol(&get_paths()[index], 50)?;
	}
	
	OK
}
