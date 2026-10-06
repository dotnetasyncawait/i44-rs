pub mod win;
pub mod audio;
pub mod xaudio2;
pub mod timer;
pub mod threadpool;

pub fn make_guid() -> Result<String, crate::common::error::OsError> {
	Ok(format!("{:?}", unsafe { windows::Win32::System::Com::CoCreateGuid()? }))
}