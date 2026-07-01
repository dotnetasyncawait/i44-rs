use std::{path::Path, sync::OnceLock};
use i44::{common::error::Error, misc::xaudio2::XAudio2};

static AUDIO: OnceLock<XAudio2> = OnceLock::new();

fn get_audio() -> &'static XAudio2 {
	AUDIO.get().expect("audio should be initialized")
}

pub fn init() {
	let audio = XAudio2::new().expect("failed to create XAudio2");
	AUDIO.set(audio).expect("sound should not be set");
}

pub fn play_vol<P: AsRef<Path>>(path: P, vol: u8) -> Result<(), Error> {
	get_audio().play_vol(path, vol)
}