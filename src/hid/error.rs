use windows::Win32::Foundation::NTSTATUS;
use windows_core::Error as Win32Error;
use crate::common::error::{OsError, tagged_error};
use std::borrow::Cow;

tagged_error!(
	Error,
	ErrorKind { DeviceNotConnected, DeviceInUse, TimedOut, Os },
	pub(in crate::hid)
);

impl Error {
	pub fn os(&self) -> Option<&OsError> { self.extra() }
	pub unsafe fn os_unchecked(&self) -> &OsError { self.extra_unchecked() }
	
	pub(super) fn new_os(ctx: impl Into<Cow<'static, str>>, err: Win32Error) -> Self {
		Self::new_custom(ErrorKind::Os, OsError::new(ctx, err))
	}
	
	pub(super) fn os_from_thread(ctx: impl Into<Cow<'static, str>>) -> Self {
		Self::new_os(ctx, Win32Error::from_thread())
	}
	
	pub(super) fn os_from_nt(ctx: impl Into<Cow<'static, str>>, nt: NTSTATUS) -> Self {
		Self::new_os(ctx, Win32Error::from_hresult(nt.to_hresult()))
	}
}

impl From<OsError> for Error {
	fn from(value: OsError) -> Self {
		Self::new_custom(ErrorKind::Os, value)
	}
}
