pub type Win32Error = windows::core::Error;

#[derive(Debug)]
pub enum Error {
	Win32(Win32Error),
	Other(String)
}

pub fn ok() -> Result<(), Error> {
	Ok(())
}

impl From<Win32Error> for Error {
	fn from(value: Win32Error) -> Self {
		Self::Win32(value)
	}
}