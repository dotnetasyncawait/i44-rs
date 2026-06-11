use windows::core::Error;

#[derive(Debug)]
pub enum HidError {
	Win32(Error),
	DeviceNotConnected,
	DeviceInUse,
	Timeout,
	Other(String)
}

impl From<Error> for HidError {
	fn from(value: Error) -> Self {
		Self::Win32(value)
	}
}

pub(super) trait Win32ErrorExt {
	fn with_context(self, msg: &str) -> Self;
}

impl Win32ErrorExt for Error {
	fn with_context(self, msg: &str) -> Self {
		Error::new(self.code(), format!("{msg}: {}", self.message()))
	}
}