use std::string::FromUtf16Error;

pub type Win32Error = windows::core::Error;
pub const OK: Result<(), Error> = Ok(());

#[derive(Debug)]
pub enum Error {
	Win32(Win32Error),
	Other(String)
}

impl From<Win32Error> for Error {
	fn from(value: Win32Error) -> Self {
		Self::Win32(value)
	}
}

impl From<FromUtf16Error> for Error {
	fn from(value: FromUtf16Error) -> Self {
		Self::Other(value.to_string())
	}
}