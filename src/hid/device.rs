use std::{fmt, slice, sync::Arc, time::Duration};
use crate::{common::{error::{Win32ErrExt, Win32ErrResExt}, native::NonNullHANDLE}};
use super::error::{Error, ErrorKind};
use windows_core::{Owned, PCWSTR};
use windows::Win32::{
	Foundation::{ERROR_DEVICE_NOT_CONNECTED, ERROR_FILE_NOT_FOUND, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING,
		ERROR_NOT_FOUND, ERROR_SHARING_VIOLATION, GENERIC_READ, GENERIC_WRITE, WAIT_TIMEOUT, WIN32_ERROR},
	Storage::FileSystem::{CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
		ReadFile, WriteFile},
	System::IO::{CancelIoEx, GetOverlappedResult, GetOverlappedResultEx, OVERLAPPED},
	System::Threading::{CreateEventW, INFINITE}};

pub enum DeviceAccess {
	Read,
	Write,
	ReadWrite
}

pub struct Device {
	info: Arc<DeviceInfo>,
	handle: Option<Owned<NonNullHANDLE>>,
	event: Option<Owned<NonNullHANDLE>>,
	input: Box<[u8]>,
	output: Box<[u8]>,
}

impl Device {
	pub fn new(info: Arc<DeviceInfo>) -> Self {
		let input = unsafe { Box::<[u8]>::new_zeroed_slice(info.input_report_byte_len as _).assume_init() };
		let output = unsafe { Box::<[u8]>::new_zeroed_slice(info.output_report_byte_len as _).assume_init() };
		
		Self { info, handle: None, event: None, input, output }
	}
	
	pub fn info(&self) -> &DeviceInfo {
		&self.info
	}
	
	pub fn open(&mut self) -> Result<(), Error> {
		self.open_with(DeviceAccess::ReadWrite)
	}
	
	pub fn open_with(&mut self, access: DeviceAccess) -> Result<(), Error> {
		if self.handle.is_some() {
			Ok(()) // TODO: return an error?
		} else {
			self.open_unchecked_with(access)
		}
	}
	
	fn open_unchecked_with(&mut self, access: DeviceAccess) -> Result<(), Error> {
		let access = match access {
			DeviceAccess::Read => GENERIC_READ,
			DeviceAccess::Write => GENERIC_WRITE,
			DeviceAccess::ReadWrite => GENERIC_READ | GENERIC_WRITE,
		};
		
		let res = unsafe {
			CreateFileW(
				PCWSTR(self.info.path.as_ptr()),
				access.0,
				FILE_SHARE_READ | FILE_SHARE_WRITE, None, OPEN_EXISTING, FILE_FLAG_OVERLAPPED, None) };
		
		let handle = match res {
			Ok(h) => unsafe { Owned::new(NonNullHANDLE::from_valid(h)) },
			Err(err) => return Err(match err.as_win32() {
				ERROR_FILE_NOT_FOUND => Error::new_simple(ErrorKind::DeviceNotConnected),
				ERROR_SHARING_VIOLATION => Error::new_simple(ErrorKind::DeviceInUse),
				_ => Error::new_os("failed to open device", err)
			})
		};
		
		if self.event.is_none() {
			let event = unsafe {
				CreateEventW(None, true, false, PCWSTR::null()).context("failed to create an event")?
			};
			self.event = Some(unsafe { Owned::new(NonNullHANDLE::from_valid(event)) });
		}
		
		self.handle = Some(handle);
		Ok(())
	}
	
	pub fn write(&mut self, output: &[u8]) -> Result<(), Error> {
		let mut must_close = if self.handle.is_some() {
			false
		} else {
			self.open_unchecked_with(DeviceAccess::Write)?;
			true
		};
		
		let buf: &mut [u8] = &mut self.output;
		let copy_len = Ord::min(output.len(), buf.len() - 1);
		buf[1..copy_len+1].copy_from_slice(&output[..copy_len]);
		
		let ret = self.write_inner(&mut must_close);
		if must_close {
			self.close();
		}
		ret
	}
	
	fn write_inner(&self, must_close: &mut bool) -> Result<(), Error> {
		let h = self.handle.as_ref().unwrap().as_handle();
		
		let mut ol = OVERLAPPED::default();
		ol.hEvent = self.event.as_ref().unwrap().as_handle();
		
		let Err(err) = (unsafe { WriteFile(h, Some(&self.output), None, Some(&mut ol)) }) else {
			// completed synchronously
			return Ok(());
		};
		
		if err.as_win32() == ERROR_IO_PENDING {
			let mut bt = 0u32;
			unsafe { GetOverlappedResult(h, &ol, &mut bt, true) }
				.map_err(|err| Self::get_write_error(err, "failed to get overlapped write-result", must_close))
		} else {
			Err(Self::get_write_error(err, "failed to write file", must_close))
		}
	}
	
	pub fn read(&mut self, input: &mut [u8]) -> Result<(), Error> {
		self.read_timeout(input, Duration::MAX)
	}
	
	pub fn read_timeout(&mut self, input: &mut [u8], timeout: Duration) -> Result<(), Error> {
		let mut must_close = if self.handle.is_some() {
			false
		} else {
			self.open_unchecked_with(DeviceAccess::Read)?;
			true
		};
		
		let res = self.read_inner(timeout.as_millis().min(INFINITE as u128) as u32, &mut must_close);
		if must_close {
			self.close();
		}
		res?;
		
		let buf: &[u8] = &self.input;
		let copy_len = Ord::min(input.len(), buf.len() - 1);
		input[..copy_len].copy_from_slice(&buf[1..copy_len+1]);
		
		Ok(())
	}
	
	fn read_inner(&mut self, timeout: u32, must_close: &mut bool) -> Result<(), Error> {
		let h = self.handle.as_ref().unwrap().as_handle();
		
		let mut ol = OVERLAPPED::default();
		ol.hEvent = self.event.as_ref().unwrap().as_handle();
		
		let Err(err) = (unsafe { ReadFile(h, Some(&mut self.input), None, Some(&mut ol)) }) else {
			// completed synchronously
			return Ok(());
		};
		
		if err.as_win32() != ERROR_IO_PENDING {
			return Err(Self::get_read_error(err, "failed to read file", must_close));
		}
		
		let mut bt = 0u32;
		let Err(err) = (unsafe { GetOverlappedResultEx(h, &ol, &mut bt, timeout, false) }) else {
			return Ok(());
		};
		
		const ERROR_WAIT_TIMEOUT: WIN32_ERROR = WIN32_ERROR(WAIT_TIMEOUT.0);
		
		if !matches!(err.as_win32(), ERROR_WAIT_TIMEOUT | ERROR_IO_INCOMPLETE) {
			return Err(Self::get_read_error(err, "failed to get overlapped read-result", must_close));
		}
		
		// timed out or 'timeout' was 0 and the operation is still in progress
		
		let Err(err) = (unsafe { CancelIoEx(h, Some(&ol)) }) else {
			return Err(Error::new_simple(ErrorKind::TimedOut));
		};
		
		match err.as_win32() {
			ERROR_NOT_FOUND => {
				// The IO operation had already been finished by the time we tried to cancel it.
				// Let's make another try to get the result.
				unsafe { GetOverlappedResult(h, &ol, &mut bt, true)
					.map_err(|err|
						Self::get_read_error(err, "failed to repeatedly get overlapped read-result", must_close)) }
			},
			_ => Err(Error::new_os("failed to cancel read IO", err))
		}
	}
	
	pub fn close(&mut self) {
		_ = self.handle.take();
	}
	
	fn get_write_error(err: windows_core::Error, ctx: &'static str, must_close: &mut bool) -> Error {
		match err.as_win32() {
			ERROR_DEVICE_NOT_CONNECTED => { *must_close = true; Error::new_simple(ErrorKind::DeviceNotConnected) },
			_ => Error::new_os(ctx, err)
		}
	}
	
	fn get_read_error(err: windows_core::Error, ctx: &'static str, must_close: &mut bool) -> Error {
		match err.as_win32() {
			ERROR_DEVICE_NOT_CONNECTED => { *must_close = true; Error::new_simple(ErrorKind::DeviceNotConnected) },
			_ => Error::new_os(ctx, err)
		}
	}
}

impl Clone for Device {
	fn clone(&self) -> Self {
		Self::new(Arc::clone(&self.info))
	}
}

#[derive(Debug)]
pub struct DeviceInfo {
	pub(super) vendor_id: u16,
	pub(super) product_id: u16,
	pub(super) usage_page: u16,
	pub(super) usage_id: u16,
	pub(super) input_report_byte_len: u16,
	pub(super) output_report_byte_len: u16,
	pub(super) path: DevicePath,
	pub(super) manufacturer: String,
	pub(super) product: String,
}

impl DeviceInfo {
	pub fn vendor_id(&self) -> u16 { self.vendor_id }
	pub fn product_id(&self) -> u16 { self.product_id }
	pub fn usage_page(&self) -> u16 { self.usage_page }
	pub fn usage_id(&self) -> u16 { self.usage_id }
	pub fn input_report_byte_len(&self) -> u16 { self.input_report_byte_len }
	pub fn output_report_byte_len(&self) -> u16 { self.output_report_byte_len }
	pub fn path(&self) -> String { self.path.to_string() }
	pub fn manufacturer(&self) -> &str { self.manufacturer.as_str() }
	pub fn product(&self) -> &str { self.product.as_str() }
}

#[derive(Default)]
pub struct DevicePath {
	/// The original `SP_DEVICE_INTERFACE_DETAIL_DATA_W` containing null-terminated, UTF-16 string.
	path: Box<[u8]>,
}

impl DevicePath {
	pub(super) fn new(path: Box<[u8]>) -> Self {
		Self { path }
	}
	
	pub(super) fn as_ptr(&self) -> *const u16 {
		self.path[4..].as_ptr() as _
	}
}

impl fmt::Debug for DevicePath {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", self.to_string())
	}
}

impl fmt::Display for DevicePath {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let size = (self.path.len() - 4) / 2 - 1; // - 4(cbSize) / 2(u8 -> u16) - 1(NULL)
		let s = String::from_utf16_lossy(unsafe { slice::from_raw_parts(self.as_ptr(), size) });
		write!(f, "{s}")
	}
}
