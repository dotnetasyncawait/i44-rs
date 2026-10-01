use std::{path::Path, sync::OnceLock};
use i44::misc::xaudio2::{XAudio2, PlayError};

static AUDIO: OnceLock<XAudio2> = OnceLock::new();

fn get_audio() -> &'static XAudio2 {
	AUDIO.get().expect("audio should be initialized")
}

pub fn init() {
	let audio = XAudio2::new().expect("failed to create XAudio2");
	AUDIO.set(audio).expect("sound should not be set");
}

pub fn play(path: impl AsRef<Path>) -> Result<(), PlayError> {
	get_audio().play(path)
}

pub fn play_vol(path: impl AsRef<Path>, vol: u8) -> Result<(), PlayError> {
	get_audio().play_vol(path, vol)
}
