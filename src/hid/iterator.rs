use super::{device::DeviceInfo, error::{HidError, Win32ErrorExt}, device::{DevicePath, HidDevice}};
use windows::core::{Error, HRESULT, Owned, PCWSTR};
use std::{mem, sync::Arc};
use windows::Win32::{
	Devices::DeviceAndDriverInstallation::{DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
		SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
		SetupDiGetDeviceInterfaceDetailW},
	Devices::HumanInterfaceDevice::{HIDD_ATTRIBUTES, HIDP_CAPS, HIDP_STATUS_SUCCESS, HidD_FreePreparsedData,
		HidD_GetAttributes, HidD_GetHidGuid, HidD_GetManufacturerString, HidD_GetPreparsedData, HidD_GetProductString,
		HidP_GetCaps, PHIDP_PREPARSED_DATA},
	Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS},
	Storage::FileSystem::{CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING},
};

pub fn enumerate() -> Result<HidDeviceIter, HidError> {
	Ok(HidDeviceIter { paths: list_paths()?, index: 0 })
}

pub struct HidDeviceIter {
	paths: Vec<DevicePath>,
	index: usize,
}

impl Iterator for HidDeviceIter {
	type Item = Result<HidDevice, HidError>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.index == self.paths.len() {
			return None;
		}
		
		let path = mem::take(&mut self.paths[self.index]);
		self.index += 1;
		
		Some(get_device(path))
	}
}

fn get_device(path: DevicePath) -> Result<HidDevice, HidError> {
	let handle = unsafe { 
		Owned::new(CreateFileW(
			PCWSTR(path.as_ptr()),
			0, // ACCESS_NONE
			FILE_SHARE_READ | FILE_SHARE_WRITE,
			None,
			OPEN_EXISTING,
			FILE_FLAGS_AND_ATTRIBUTES(0),
			None).map_err(|err| err.with_context(&format!("Failed to open device {path:?}")))?) };
	
	let mut attributes = HIDD_ATTRIBUTES::default();
	if unsafe { !HidD_GetAttributes(*handle, &mut attributes) } {
		Err(Error::from_thread().with_context(&format!("Failed to get attributes {path:?}")))?;
	}
	
	let mut data = PHIDP_PREPARSED_DATA::default();
	if unsafe { !HidD_GetPreparsedData(*handle, &mut data) } {
		Err(Error::from_thread().with_context(&format!("Failed to get preparsed data {path:?}")))?;
	}
	
	let mut caps = HIDP_CAPS::default();
	let status = unsafe { HidP_GetCaps(data, &mut caps) };
	
	if status != HIDP_STATUS_SUCCESS {
		assert!(unsafe { HidD_FreePreparsedData(data) });
		Err(Error::from_hresult(status.to_hresult()).with_context(&format!("Failed to get caps {path:?}")))?;
	}
	
	assert!(unsafe { HidD_FreePreparsedData(data) });
	
	// According to the docs, the maximum string length for USB devices
	// is 126 wide characters + 1 terminating NULL-character.
	// TODO: what if non-USB device?
	const BUFFER_SIZE: u32 = 127;
	let mut buff = [0u16; BUFFER_SIZE as _];
	
	if unsafe { !HidD_GetManufacturerString(*handle, buff.as_mut_ptr() as _, BUFFER_SIZE) } {
		Err(Error::from_thread().with_context(&format!("Failed to get manufacturer string {path:?}")))?;
	}
	let manufacturer = get_str(&mut buff);
	
	if unsafe { !HidD_GetProductString(*handle, buff.as_mut_ptr() as _, BUFFER_SIZE) } {
		Err(Error::from_thread().with_context(&format!("Failed to get product string {path:?}")))?;
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
	
	return Ok(HidDevice { info: Arc::new(info) });
	
	fn get_str(buff: &mut [u16]) -> String {
		let index = buff.iter().position(|&ch| ch == 0).expect("buffer should be null-terminated");
		let ret = String::from_utf16(&buff[..index]).expect("buffer should be valid UTF-16 string");
		buff[0] = 0;
		ret
	}
}

fn list_paths() -> Result<Vec<DevicePath>, HidError> {
	let guid = unsafe { HidD_GetHidGuid() };
	
	let h_devinfo = unsafe {
		Owned::new(SetupDiGetClassDevsW(
			Some(&guid),
			PCWSTR::null(),
			None,
			DIGCF_PRESENT | DIGCF_DEVICEINTERFACE
			).map_err(|err| err.with_context("Failed to get Device Information Set"))?) };
	
	let mut dev_idata = SP_DEVICE_INTERFACE_DATA::default();
	dev_idata.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as _;
	
	let mut paths = Vec::<DevicePath>::new();
	let mut index: u32 = 0;
	
	loop {
		const NO_MORE_ITEMS: HRESULT = HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0);
		
		if let Err(err) = unsafe { SetupDiEnumDeviceInterfaces(*h_devinfo, None, &guid, index, &mut dev_idata) } {
			match err.code() {
				NO_MORE_ITEMS => break,
				_ => Err(err.with_context("Failed to enumerate Device Interfaces"))?
			}
		}
		
		index += 1;
		
		const INSUFFICIENT_BUFFER: HRESULT = HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0);
		let mut required_size: u32 = 0;
		
		let err = unsafe {
			SetupDiGetDeviceInterfaceDetailW(*h_devinfo, &dev_idata, None, 0, Some(&mut required_size), None)
				.expect_err("First call should return an error") };
		
		if err.code() != INSUFFICIENT_BUFFER {
			Err(err.with_context("Failed to get Device Interface Detail size"))?;
		}
		
		let mut buff = vec![0u8; required_size as _];
		let detail = buff.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
		unsafe { (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as _ }
		
		unsafe {
			SetupDiGetDeviceInterfaceDetailW(*h_devinfo, &dev_idata, Some(detail), buff.len() as _, None, None)
				.map_err(|err| err.with_context("Failed to get Device Interface Detail"))?
		}
		
		paths.push(DevicePath { path: buff })
	}
	
	Ok(paths)
}