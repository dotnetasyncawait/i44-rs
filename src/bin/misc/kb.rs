use std::sync::{Arc, LazyLock};
use i44::hid::{self, DeviceInfo, HidDevice, HidError};
use hid_msgs::*;
use layers::*;

type HidResult = Result<(), HidError>;

pub mod layers {
	pub const NORMAL: u8 = 0;
	pub const INSERT: u8 = 1;
	pub const SELECT: u8 = 2;
	pub const SYMBOL: u8 = 3;
	pub const U_SYMB: u8 = 4;
	pub const MOUSE:  u8 = 5;
	pub const SYSTEM: u8 = 6;
}

#[allow(unused)]
pub mod hid_msgs {
	pub const RESERVED:         u8 = 0x00;
	pub const HID_HOST:         u8 = 0x01;
	pub const HID_SET_LAYER:    u8 = 0x02;
	pub const HID_GET_LAYER:    u8 = 0x03;
	pub const HID_LAYER_UPDATE: u8 = 0x04;
	pub const HID_DEFAULT:      u8 = 0xFE;
	pub const HID_PING:         u8 = 0xFF;
}

static KB: LazyLock<I44> = LazyLock::new(|| {
	let info = hid::enumerate()
		.expect("hid enumeration should not fail")
		.filter_map(|r| r.ok())
		.filter(|di| di.vendor_id() == 0xFEED && di.product_id() == 0x03)
		.filter(|di| di.usage_page() == 0xFF60 && di.usage_id() == 0x61)
		.nth(0)
		.expect("i44 should be present");

	I44 { info: Arc::new(info) }
});

pub struct I44 {
	info: Arc<DeviceInfo>,
}

impl I44 {
	pub fn new_device() -> HidDevice {
		HidDevice::new(Arc::clone(&KB.info))
	}
	
	pub fn enable() -> HidResult {
		Self::new_device().write(&[HID_HOST, 1])
	}
	
	pub fn disable() -> HidResult {
		Self::new_device().write(&[HID_HOST, 0])
	}
	
	pub fn set_mouse_layer() -> HidResult {
		Self::set_layer(1 << MOUSE)
	}
	
	pub fn set_layer(layer: u16) -> HidResult {
		Self::new_device().write(&[HID_SET_LAYER, ((layer >> 8) & 0xFF) as u8, (layer & 0xFF) as u8])
	}
}
