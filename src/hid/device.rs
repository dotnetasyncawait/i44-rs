use std::{slice, sync::Arc, fmt::{self, Debug}};

#[derive(Clone)]
pub struct HidDevice {
	pub(super) info: Arc<DeviceInfo>
}

impl HidDevice {
	pub fn vendor_id(&self) -> u16 { self.info.vendor_id }
	pub fn product_id(&self) -> u16 { self.info.product_id }
	pub fn usage_page(&self) -> u16 { self.info.usage_page }
	pub fn usage_id(&self) -> u16 { self.info.usage_id }
	pub fn input_report_byte_len(&self) -> u16 { self.info.input_report_byte_len }
	pub fn output_report_byte_len(&self) -> u16 { self.info.output_report_byte_len }
	pub fn path(&self) -> String { self.info.path.to_string() }
	pub fn manufacturer(&'_ self) -> &'_ str { self.info.manufacturer.as_str() }
	pub fn product(&'_ self) -> &'_ str { self.info.product.as_str() }
	
	// open
	// read
	// write
	// close
}

#[derive(Debug)]
pub(super) struct DeviceInfo {
	pub vendor_id: u16,
	pub product_id: u16,
	pub usage_page: u16,
	pub usage_id: u16,
	pub input_report_byte_len: u16,
	pub output_report_byte_len: u16,
	pub path: DevicePath,
	pub manufacturer: String,
	pub product: String,
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