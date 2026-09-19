use std::{borrow::Cow, error::Error as StdError, fmt};
use windows_core::{Error as Win32Error, HRESULT};
use windows::Win32::Foundation::WIN32_ERROR;

pub const OK: Result<(), Error> = Ok(());

#[derive(Debug)]
pub struct Error {
	inner: Box<dyn StdError + 'static>,
}

impl<E: StdError + 'static> From<E> for Error {
	fn from(value: E) -> Self {
		Self { inner: Box::new(value) }
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.inner.fmt(f)
	}
}

#[derive(Debug)]
pub struct OsError {
	inner: Box<Inner>
}

#[derive(Debug)]
struct Inner {
	ctx: Cow<'static, str>,
	err: Win32Error,
}

impl OsError {
	pub fn from_hr(ctx: impl Into<Cow<'static, str>>, hr: HRESULT) -> Self {
		Self::from_err(ctx, Win32Error::from_hresult(hr))
	}
	
	pub fn from_win32(ctx: impl Into<Cow<'static, str>>, code: WIN32_ERROR) -> Self {
		Self::from_err(ctx, Win32Error::from(code))
	}
	
	pub fn from_thread(ctx: impl Into<Cow<'static, str>>) -> Self {
		Self::from_err(ctx, Win32Error::from_thread())
	}
	
	pub fn from_err(ctx: impl Into<Cow<'static, str>>, err: Win32Error) -> Self {
		Self { inner: Box::new(Inner { ctx: ctx.into(), err }) }
	}
	
	pub fn code(&self) -> HRESULT { self.inner.err.code() }
}

impl StdError for OsError {}

impl fmt::Display for OsError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.inner.ctx.is_empty() {
			self.inner.err.fmt(f)
		} else {
			write!(f, "{}: {}", self.inner.ctx, self.inner.err)
		}
	}
}

impl From<Win32Error> for OsError {
	fn from(value: Win32Error) -> Self {
		Self::from_err("", value)
	}
}

pub(crate) trait Win32ErrResExt {
	type OK;
	fn context(self, ctx: impl Into<Cow<'static, str>>) -> Result<Self::OK, OsError>;
	fn with_context<R: Into<Cow<'static, str>>>(self, ctx: impl FnOnce() -> R) -> Result<Self::OK, OsError>;
}

impl<T> Win32ErrResExt for Result<T, Win32Error> {
	type OK = T;
	
	fn context(self, ctx: impl Into<Cow<'static, str>>) -> Result<T, OsError> {
		self.map_err(|err| OsError::from_err(ctx, err))
	}
	
	fn with_context<C: Into<Cow<'static, str>>>(self, ctx: impl FnOnce() -> C) -> Result<T, OsError> {
		self.map_err(|err| OsError::from_err(ctx(), err))
	}
}

pub(crate) trait Win32ErrExt {
	fn as_win32(&self) -> WIN32_ERROR;
}

impl Win32ErrExt for Win32Error {
	fn as_win32(&self) -> WIN32_ERROR {
		WIN32_ERROR(self.code().0 as u32 & 0xFFFF)
	}
}
