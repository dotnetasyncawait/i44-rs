use std::{fmt::{self, Debug}, slice, sync::Arc, time::Duration};
use super::{HidError, error::Win32ErrorExt};
use windows::core::{Error, HRESULT, Owned, PCWSTR};
use windows::Win32::{
	Foundation::{HANDLE, ERROR_DEVICE_NOT_CONNECTED, ERROR_FILE_NOT_FOUND, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING,
		ERROR_NOT_FOUND, ERROR_SHARING_VIOLATION, GENERIC_READ, GENERIC_WRITE, WAIT_TIMEOUT},
	Storage::FileSystem::{CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
		ReadFile, WriteFile},
	System::IO::{CancelIoEx, GetOverlappedResult, GetOverlappedResultEx, OVERLAPPED},
	System::Threading::{CreateEventW, INFINITE}};

pub enum DeviceAccess {
	Read,
	Write,
	ReadWrite
}

pub struct HidDevice {
	info: Arc<DeviceInfo>,
	handle: Option<Owned<HANDLE>>,
	event: Option<Owned<HANDLE>>,
	input: Vec<u8>,
	output: Vec<u8>,
}

impl HidDevice {
	pub fn new(info: Arc<DeviceInfo>) -> Self {
		let input_len = info.input_report_byte_len;
		let output_len = info.output_report_byte_len;
		
		Self {
			info,
			handle: None,
			event: None,
			input: vec![0u8; input_len as _],
			output: vec![0u8; output_len as _],
		}
	}
	
	pub fn info(&self) -> &DeviceInfo {
		&self.info
	}
	
	pub fn open(&mut self) -> Result<(), HidError> {
		self.open_with(DeviceAccess::ReadWrite)
	}
	
	pub fn open_with(&mut self, access: DeviceAccess) -> Result<(), HidError> {
		if let Some(_) = self.handle {
			return Ok(()); // TODO: return an error?
		}
		
		let access = match access {
			DeviceAccess::Read => GENERIC_READ,
			DeviceAccess::Write => GENERIC_WRITE,
			DeviceAccess::ReadWrite => GENERIC_READ | GENERIC_WRITE,
		};
		
		let res: Result<HANDLE, Error> = unsafe { 
			CreateFileW(
				PCWSTR(self.info.path.as_ptr()),
				access.0,
				FILE_SHARE_READ | FILE_SHARE_WRITE,
				None,
				OPEN_EXISTING,
				FILE_FLAG_OVERLAPPED,
				None) };
		
		let handle = match res {
			Ok(h) => unsafe { Owned::new(h) },
			Err(err) => {
				const FILE_NOT_FOUND: HRESULT = HRESULT::from_win32(ERROR_FILE_NOT_FOUND.0);
				const SHARING_VIOLATION: HRESULT = HRESULT::from_win32(ERROR_SHARING_VIOLATION.0);
				
				return Err(match err.code() {
					FILE_NOT_FOUND => HidError::DeviceNotConnected,
					SHARING_VIOLATION => HidError::DeviceInUse,
					_ => err.into()
				});
			}
		};
		
		if self.event.is_none() {
			let event = unsafe {
				Owned::new(CreateEventW(None, true, false, PCWSTR::null())
					.map_err(|err| err.with_context("Failed to create an event"))?) };
			
			self.event = Some(event);
		}
		
		self.handle = Some(handle);
		Ok(())
	}
	
	pub fn write(&mut self, output: &[u8]) -> Result<(), HidError> {
		let (h, must_close) = match &self.handle {
			Some(owned) => (**owned, false),
			None => {
				self.open_with(DeviceAccess::Write)?;
				(**self.handle.as_ref().unwrap(), true)
			}
		};
		
		let event = *self.event.as_deref().unwrap();
		let buffer = &mut self.output;
		
		let copy_len = Ord::min(output.len(), buffer.len() - 1);
		buffer[1..copy_len+1].copy_from_slice(&output[..copy_len]);
		
		let ret = Self::write_inner(h, event, &buffer);
		if must_close {
			self.close();
		}
		ret
	}
	
	fn write_inner(h: HANDLE, event: HANDLE, output: &[u8]) -> Result<(), HidError> {
		let mut ol = OVERLAPPED::default();
		ol.hEvent = event;
		
		let Err(err) = (unsafe { WriteFile(h, Some(output), None, Some(&mut ol)) }) else {
			// completed synchronously
			return Ok(());
		};
		
		if err.code() != HRESULT::from_win32(ERROR_IO_PENDING.0) {
			return Err(Self::get_write_error(err));
		}
		
		let mut bt = 0u32;
		if let Err(err) = unsafe { GetOverlappedResult(h, &ol, &mut bt, true) } {
			return Err(Self::get_write_error(err));
		}
		
		Ok(())
	}
	
	pub fn read(&mut self, input: &mut [u8]) -> Result<(), HidError> {
		self.read_timeout(input, Duration::MAX)
	}
	
	pub fn read_timeout(&mut self, input: &mut [u8], timeout: Duration) -> Result<(), HidError> {
		let (h, must_close) = match &self.handle {
			Some(owned) => (**owned, false),
			None => {
				self.open_with(DeviceAccess::Read)?;
				(**self.handle.as_ref().unwrap(), true)
			}
		};
		
		let event = *self.event.as_deref().unwrap();
		let buff = &mut self.input;
		let timeout = timeout.as_millis().min(INFINITE as u128) as u32;
		
		let ret = Self::read_inner(h, event, buff, timeout);
		if must_close {
			self.close();
		}
		ret?;
		
		let buffer = &self.input;
		
		let copy_len = Ord::min(input.len(), buffer.len() - 1);
		input[..copy_len].copy_from_slice(&buffer[1..copy_len+1]);
		
		Ok(())
	}
	
	fn read_inner(h: HANDLE, event: HANDLE, buffer: &mut [u8], timeout: u32) -> Result<(), HidError> {
		let mut ol = OVERLAPPED::default();
		ol.hEvent = event;
		
		let Err(err) = (unsafe { ReadFile(h, Some(buffer), None, Some(&mut ol)) }) else {
			// completed synchronously
			return Ok(());
		};
		
		if err.code() != HRESULT::from_win32(ERROR_IO_PENDING.0) {
			return Err(Self::get_read_error(err));
		}
		
		let mut bt = 0u32;
		let Err(err) = (unsafe { GetOverlappedResultEx(h, &ol, &mut bt, timeout, false) }) else {
			return Ok(());
		};
		
		const TIMEOUT: HRESULT = HRESULT::from_win32(WAIT_TIMEOUT.0);
		const IO_INCOMPLETE: HRESULT = HRESULT::from_win32(ERROR_IO_INCOMPLETE.0);
		
		if !matches!(err.code(), TIMEOUT | IO_INCOMPLETE) {
			return Err(Self::get_read_error(err));
		}
		
		// timed out or 'timeout' was 0 and the operation is still in progress
		
		let Err(err) = (unsafe { CancelIoEx(h, Some(&ol)) }) else {
			return Err(HidError::Timeout);
		};
		
		const NOT_FOUND: HRESULT = HRESULT::from_win32(ERROR_NOT_FOUND.0);
		
		match err.code() {
			NOT_FOUND => {
				// The IO operation had already been finished by the time we tried to cancel it.
				// Let's make another try to get the result.
				match unsafe { GetOverlappedResult(h, &ol, &mut bt, true) } {
					Ok(_) => Ok(()),
					Err(err) => Err(Self::get_read_error(err))
				}
			},
			_ => Err(err.with_context("Failed to cancel read IO").into())
		}
	}
	
	pub fn close(&mut self) {
		let _ = self.handle.take();
	}
	
	fn get_write_error(err: Error) -> HidError {
		const DEVICE_NOT_CONNECTED: HRESULT = HRESULT::from_win32(ERROR_DEVICE_NOT_CONNECTED.0);
		
		match err.code() {
			DEVICE_NOT_CONNECTED => HidError::DeviceNotConnected,
			_ => err.with_context("Failed to write").into()
		}
	}
	
	fn get_read_error(err: Error) -> HidError {
		const DEVICE_NOT_CONNECTED: HRESULT = HRESULT::from_win32(ERROR_DEVICE_NOT_CONNECTED.0);
		
		match err.code() {
			DEVICE_NOT_CONNECTED => HidError::DeviceNotConnected,
			_ => err.with_context("Failed to read").into()
		}
	}
}

impl Clone for HidDevice {
	fn clone(&self) -> Self {
		let input_len = self.info.input_report_byte_len;
		let output_len = self.info.output_report_byte_len;
		
		Self {
			info: Arc::clone(&self.info),
			handle: None,
			event: None,
			input: vec![0u8; input_len as _],
			output: vec![0u8; output_len as _],
		}
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
pub(super) struct DevicePath {
	// The original SP_DEVICE_INTERFACE_DETAIL_DATA_W containing null-terminated, UTF-16 string.
	pub path: Vec<u8>,
}

impl DevicePath {
	pub fn as_ptr(&self) -> *const u16 {
		self.path[4..].as_ptr() as _
	}
}

impl ToString for DevicePath {
	fn to_string(&self) -> String {
		let size = ((self.path.len() - 4) / 2 - 1) as usize; // - 4(cbSize) / 2(u8 -> u16) - 1(NULL)
		String::from_utf16(unsafe { slice::from_raw_parts(self.as_ptr(), size) })
			.expect("device path should be valid UTF-16")
	}
}

impl Debug for DevicePath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&format!("{:?}", self.to_string()))
	}
}