use crate::common::error::{Win32ErrExt, Win32ErrResExt};
use super::{device::{DeviceInfo, DevicePath}, error::{Error, ErrorKind}};
use windows::core::{Owned, PCWSTR};
use std::mem;
use windows::Win32::{
	Devices::DeviceAndDriverInstallation::{DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
		SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
		SetupDiGetDeviceInterfaceDetailW},
	Devices::HumanInterfaceDevice::{HIDD_ATTRIBUTES, HIDP_CAPS, HIDP_STATUS_SUCCESS, HidD_FreePreparsedData,
		HidD_GetAttributes, HidD_GetHidGuid, HidD_GetManufacturerString, HidD_GetPreparsedData, HidD_GetProductString,
		HidP_GetCaps, PHIDP_PREPARSED_DATA},
	Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS, ERROR_SHARING_VIOLATION},
	Storage::FileSystem::{CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING},
};

pub fn devices() -> Result<DeviceIter, Error> {
	Ok(DeviceIter { paths: list_paths()?, index: 0 })
}

pub struct DeviceIter {
	paths: Box<[DevicePath]>,
	index: usize,
}

impl Iterator for DeviceIter {
	type Item = Result<DeviceInfo, (Error, DevicePath)>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.index == self.paths.len() {
			return None;
		}
		
		let path = mem::take(&mut self.paths[self.index]);
		self.index += 1;
		
		Some(get_device(path))
	}
}

fn get_device(path: DevicePath) -> Result<DeviceInfo, (Error, DevicePath)> {
	let res = unsafe { CreateFileW(
		PCWSTR(path.as_ptr()),
		0, // ACCESS_NONE
		FILE_SHARE_READ | FILE_SHARE_WRITE,
		None,
		OPEN_EXISTING,
		FILE_FLAGS_AND_ATTRIBUTES(0), None)
	};
	
	let hdev = match res {
		Ok(h) => unsafe { Owned::new(h) },
		Err(err) => return Err(match err.as_win32() {
			ERROR_SHARING_VIOLATION => (Error::new_simple(ErrorKind::DeviceInUse), path),
			_ => (Error::new_os("failed to open device", err), path)
		})
	};
	
	let mut attributes = HIDD_ATTRIBUTES::default();
	if unsafe { !HidD_GetAttributes(*hdev, &mut attributes) } {
		return Err((Error::os_from_thread("failed to get attributes"), path));
	}
	
	let mut data = PHIDP_PREPARSED_DATA::default();
	if unsafe { !HidD_GetPreparsedData(*hdev, &mut data) } {
		return Err((Error::os_from_thread("failed to get preparsed data"), path));
	}
	let data = PreparsedDataFreeGuard(data);
	
	let mut caps = HIDP_CAPS::default();
	let status = unsafe { HidP_GetCaps(data.0, &mut caps) };
	
	if status != HIDP_STATUS_SUCCESS {
		return Err((Error::os_from_nt("failed to get caps", status), path));
	}
	
	drop(data);
	
	// According to the docs, the maximum string length for USB devices
	// is 126 wide characters + 1 terminating NULL-character.
	// TODO: what if non-USB device?
	const BUFFER_SIZE: u32 = 127;
	let mut buff = [0u16; BUFFER_SIZE as _];
	
	if unsafe { !HidD_GetManufacturerString(*hdev, buff.as_mut_ptr() as _, BUFFER_SIZE) } {
		return Err((Error::os_from_thread("failed to get manufacturer string"), path));
	}
	let manufacturer = get_str(&mut buff);
	
	if unsafe { !HidD_GetProductString(*hdev, buff.as_mut_ptr() as _, BUFFER_SIZE) } {
		return Err((Error::os_from_thread("failed to get product string"), path));
	}
	let product = get_str(&mut buff);
	
	let info = DeviceInfo {
		vendor_id: attributes.VendorID,
		product_id: attributes.ProductID,
		usage_page: caps.UsagePage,
		usage_id: caps.Usage,
		input_report_byte_len: caps.InputReportByteLength,
		output_report_byte_len: caps.OutputReportByteLength,
		path,
		manufacturer,
		product
	};	
	
	return Ok(info);
	
	fn get_str(buff: &mut [u16]) -> String {
		let index = buff.iter().position(|&ch| ch == 0).expect("buffer should be null-terminated");
		let ret = String::from_utf16_lossy(&buff[..index]);
		buff[0] = 0;
		ret
	}
	
	struct PreparsedDataFreeGuard(PHIDP_PREPARSED_DATA);
	
	impl Drop for PreparsedDataFreeGuard {
		fn drop(&mut self) {
			_ = unsafe { HidD_FreePreparsedData(self.0) };
		}
	}
}

fn list_paths() -> Result<Box<[DevicePath]>, Error> {
	let guid = unsafe { HidD_GetHidGuid() };
	
	let h_devinfo = unsafe {
		Owned::new(SetupDiGetClassDevsW(
			Some(&guid),
			PCWSTR::null(),
			None,
			DIGCF_PRESENT | DIGCF_DEVICEINTERFACE).context("failed to get Device Information Set")?) };
	
	let mut dev_idata = SP_DEVICE_INTERFACE_DATA::default();
	dev_idata.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as _;
	
	let mut paths = Vec::<DevicePath>::new();
	let mut index: u32 = 0;
	
	loop {
		if let Err(err) = unsafe { SetupDiEnumDeviceInterfaces(*h_devinfo, None, &guid, index, &mut dev_idata) } {
			match err.as_win32() {
				ERROR_NO_MORE_ITEMS => break,
				_ => return Err(Error::new_os("failed to enumerate Device Interfaces", err))
			}
		}
		
		index += 1;
		let mut required_size: u32 = 0;
		
		let err = unsafe {
			SetupDiGetDeviceInterfaceDetailW(*h_devinfo, &dev_idata, None, 0, Some(&mut required_size), None)
				.expect_err("first call should return an error") };
		
		if err.as_win32() != ERROR_INSUFFICIENT_BUFFER {
			return Err(Error::new_os("failed to get Device Interface Detail size", err));
		}
		
		let mut buf = Box::<[u8]>::new_uninit_slice(required_size as _);
		let detail = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
		
		unsafe {
			(*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as _;
			SetupDiGetDeviceInterfaceDetailW(*h_devinfo, &dev_idata, Some(detail), buf.len() as _, None, None)
				.context("failed to get Device Interface Detail")?;
		}
		
		paths.push(DevicePath::new(unsafe { buf.assume_init() }));
	}
	
	Ok(paths.into_boxed_slice())
}