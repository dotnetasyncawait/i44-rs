use std::{ptr, fmt};
use crate::common::error::{Error, ErrResultExt, Win32Error};
use windows::core::{Interface, implement};
use windows::Win32::{
	Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
	System::{Variant::VT_LPWSTR, Com::{CLSCTX_ALL, CoCreateInstance, STGM_READ}},
	Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl},
	Media::Audio::{DEVICE_STATE, DEVICE_STATE_ACTIVE, DEVICE_STATE_DISABLED, DEVICE_STATE_NOTPRESENT,
		DEVICE_STATE_UNPLUGGED, DEVICE_STATEMASK_ALL, EDataFlow, IMMDevice, IMMDeviceCollection, IMMDeviceEnumerator,
		IMMEndpoint, MMDeviceEnumerator, eConsole, eRender, AUDIO_VOLUME_NOTIFICATION_DATA}};

type VolumeNotfHandler = fn(&VolumeNotfEvent) -> Result<(), Error>;

pub fn default_device(d_type: DeviceType) -> Result<Device, Error> {
	let d = unsafe { device_enumerator()?.GetDefaultAudioEndpoint(EDataFlow(d_type as _), eConsole)? };
	Device::new(d, Some(d_type))
}

pub fn enum_devices(scope: DeviceScope, state: DeviceStates) -> Result<DeviceIter, Error> {
	Ok(DeviceIter::new(scope, state).with_context(|| "Failed to create device iterator")?)
}

pub struct Device {
	name: String,
	d_type: DeviceType,
	d: IMMDevice,
	volume: IAudioEndpointVolume,
	callback: Option<IAudioEndpointVolumeCallback>,
}

impl Device {	
	fn new(d: IMMDevice, d_type: Option<DeviceType>) -> Result<Self, Error> {
		unsafe {
			let store = d.OpenPropertyStore(STGM_READ).with_context(|| "Failed to open PropertyStore")?;
			let prop = store.GetValue(&PKEY_Device_FriendlyName).with_context(|| "Failed to get FriendlyName")?;
			
			assert_eq!(prop.vt(), VT_LPWSTR);
			let name = prop.Anonymous.Anonymous.Anonymous.pwszVal
				.to_string()
				.expect("PKEY_Device_FriendlyName should be valid UTF-16 string");
			
			let d_type = match d_type {
				Some(t) => t,
				None => {
					let endpoint: IMMEndpoint = d.cast().expect("QueryInterface::IMMEndpoint should not fail");
					let flow = endpoint.GetDataFlow().with_context(|| "Failed to get data flow")?;
					if flow == eRender { DeviceType::Render } else { DeviceType::Capture }
				}
			};
			
			let volume: IAudioEndpointVolume = d.Activate(CLSCTX_ALL, None)?;
			Ok(Self { name, d_type, d, volume, callback: None })
		}
	}
	
	pub fn name(&self) -> &str { &self.name }
	
	pub fn is_render(&self) -> bool { self.d_type == DeviceType::Render }
	
	pub fn is_capture(&self) -> bool { self.d_type == DeviceType::Capture }
	
	pub fn state(&self) -> Result<DeviceState, Error> {
		Ok(match unsafe { self.d.GetState()? } {
			DEVICE_STATE_ACTIVE => DeviceState::Active,
			DEVICE_STATE_DISABLED => DeviceState::Disabled,
			DEVICE_STATE_NOTPRESENT => DeviceState::NotPresent,
			DEVICE_STATE_UNPLUGGED => DeviceState::Unplugged,
			_ => unreachable!()
		})
	}
	
	pub fn set_mute(&self, mute: bool) -> Result<(), Error> {
		unsafe { Ok(self.volume.SetMute(mute, ptr::null())?) }
	}
	
	pub fn get_mute(&self) -> Result<bool, Error> {
		unsafe { Ok(self.volume.GetMute()?.as_bool()) }
	}
	
	pub fn tgl_mute(&self) -> Result<bool, Error> {
		let state = self.get_mute()?;
		self.set_mute(!state)?;
		Ok(!state)
	}
	
	pub fn set_volume(&self, vol: i8) -> Result<(), Error> {
		unsafe { Ok(self.volume.SetMasterVolumeLevelScalar(vol.clamp(0, 100) as f32 / 100f32, ptr::null())?) }
	}
	
	pub fn get_volume(&self) -> Result<i8, Error> {
		unsafe { Ok((self.volume.GetMasterVolumeLevelScalar()? * 100f32).round() as i8) }
	}
	
	pub fn on_volume_update(&mut self, f: VolumeNotfHandler) -> Result<(), Error> {
		let callback: IAudioEndpointVolumeCallback = AudioEndpointVolumeCallback(f).into();
		unsafe { self.volume.RegisterControlChangeNotify(&callback)? };
		self.callback = Some(callback);
		Ok(())
	}
}

impl fmt::Debug for Device {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Device")
			.field("name", &self.name)
			.field("type", &self.d_type)
			.finish()
	}
}

impl Drop for Device {
	fn drop(&mut self) {
		if let Some(c) = self.callback.take() {
			unsafe { self.volume.UnregisterControlChangeNotify(&c).expect("failed to unregister volume callback") };
		}
	}
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
	Render,
	Capture
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceScope {
	Render,
	Capture,
	All
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
	Active,
	Disabled,
	NotPresent,
	Unplugged
}

pub struct DeviceStates(u32);

impl DeviceStates {
	pub const ACTIVE: Self = Self(DEVICE_STATE_ACTIVE.0);
	pub const DISABLED: Self = Self(DEVICE_STATE_DISABLED.0);
	pub const NOTPRESENT: Self = Self(DEVICE_STATE_NOTPRESENT.0);
	pub const UNPLUGGED: Self = Self(DEVICE_STATE_UNPLUGGED.0);
	pub const ALL: Self = Self(DEVICE_STATEMASK_ALL);
	
	pub const ACTIVE_OR_DISABLED: Self = Self::ACTIVE.or(Self::DISABLED);
	pub const ACTIVE_OR_NOTPRESENT: Self = Self::ACTIVE.or(Self::NOTPRESENT);
	pub const ACTIVE_OR_UNPLUGGED: Self = Self::ACTIVE.or(Self::UNPLUGGED);
	pub const DISABLED_OR_NOTPRESENT: Self = Self::DISABLED.or(Self::NOTPRESENT);
	pub const DISABLED_OR_UNPLUGGED: Self = Self::DISABLED.or(Self::UNPLUGGED);
	pub const NOTPRESENT_OR_UNPLUGGED: Self = Self::NOTPRESENT.or(Self::UNPLUGGED);
	
	pub const ACTIVE_OR_DISABLED_OR_NOTPRESENT: Self = Self::ACTIVE_OR_DISABLED.or(Self::NOTPRESENT);
	pub const ACTIVE_OR_DISABLED_OR_UNPLUGGED: Self = Self::ACTIVE_OR_DISABLED.or(Self::UNPLUGGED);
	pub const ACTIVE_OR_NOTPRESENT_OR_UNPLUGGED: Self = Self::ACTIVE_OR_NOTPRESENT.or(Self::UNPLUGGED);
	pub const DISABLED_OR_NOTPRESENT_OR_UNPLUGGED: Self = Self::DISABLED_OR_NOTPRESENT.or(Self::UNPLUGGED);
	
	const fn or(self, rhs: Self) -> Self {
		Self(self.0 | rhs.0)
	}
}

pub struct DeviceIter {
	collection: Option<IMMDeviceCollection>,
	count: u32,
	index: u32,
	scope: DeviceScope,
}

impl DeviceIter {
	fn new(scope: DeviceScope, state: DeviceStates) -> Result<Self, Win32Error> {
		unsafe {
			let collection = device_enumerator()?.EnumAudioEndpoints(EDataFlow(scope as _), DEVICE_STATE(state.0))?;
			let count = collection.GetCount()?;
			Ok(Self { collection: if count > 0 { Some(collection) } else { None }, count, index: 0, scope })
		}
	}
}

impl Iterator for DeviceIter {
	type Item = Result<Device, Error>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.count == 0 {
			debug_assert!(self.collection.is_none());
			return None;
		}
		
		let collection = self.collection.as_ref().expect("collection should be Some if index < count");
		let d_res = unsafe { collection.Item(self.index) };
		
		self.index += 1;
		if self.index == self.count {
			self.index = 0;
			self.count = 0;
			self.collection = None;
		}
		
		Some(match d_res {
			Ok(d) => {
				let d_type = match self.scope {
					DeviceScope::Render => Some(DeviceType::Render),
					DeviceScope::Capture => Some(DeviceType::Capture),
					DeviceScope::All => None
				};
				Device::new(d, d_type)
			},
			Err(err) => Err(err.into())
		})
	}
}

fn device_enumerator() -> Result<IMMDeviceEnumerator, Win32Error> {
	unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

pub struct VolumeNotfEvent {
	pub vol: u8,
	pub muted: bool,
	pub ctx: u128,
}

impl fmt::Debug for VolumeNotfEvent {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("VolumeNotfEvent")
			.field("vol", &self.vol)
			.field("muted", &self.muted)
			.field("ctx", &format!("{{{:?}}}", windows::core::GUID::from_u128(self.ctx)))
			.finish()
	}
}

#[implement(IAudioEndpointVolumeCallback)]
pub struct AudioEndpointVolumeCallback(VolumeNotfHandler);

impl IAudioEndpointVolumeCallback_Impl for AudioEndpointVolumeCallback_Impl {
	fn OnNotify(&self, notify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<(), Win32Error> {
		let n = unsafe { &*notify };
		
		let event = VolumeNotfEvent {
			vol: (n.fMasterVolume * 100f32).round() as u8,
			muted: n.bMuted.as_bool(),
			ctx: n.guidEventContext.to_u128(),
		};
		
		if let Err(err) = (self.0)(&event) {
			println!("From IAudioEndpointVolumeCallback: {err:?}"); // TODO: display with window
		}
		
		Ok(())
	}
}