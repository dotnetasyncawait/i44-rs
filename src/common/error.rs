use std::{fmt::Display, string::FromUtf16Error};
use crate::hid::HidError;

pub type Win32Error = windows::core::Error;
pub const OK: Result<(), Error> = Ok(());

#[derive(Debug)]
pub enum Error {
	Win32(Win32Error),
	Hid(HidError),
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

impl From<HidError> for Error {
	fn from(value: HidError) -> Self {
		Self::Hid(value)
	}
}

pub trait ErrResultExt<F: FnOnce() -> S, S> {
	fn with_context(self, ctx: F) -> Self;
}

impl<T, F, S> ErrResultExt<F, S> for Result<T, Win32Error>
	where F: FnOnce() -> S, S: Display
{
	fn with_context(self, ctx: F) -> Self {
		match self {
			Ok(ok) => Ok(ok),
			Err(err) => Err(Win32Error::new(err.code(), format!("{}: {}", ctx(), err.message())))
		}
	}
}

pub trait Win32ErrExt<E> {
	fn with_context(self, ctx: E) -> Self;
}

impl<E: Display> Win32ErrExt<E> for Win32Error {
	fn with_context(self, ctx: E) -> Self {
		Win32Error::new(self.code(), format!("{}: {}", ctx, self.message()))
	}
}