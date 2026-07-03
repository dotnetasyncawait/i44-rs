use std::path::Path;
use std::os::windows::ffi::OsStrExt;
use crate::common::error::Error;
use crate::input::{mods::Mods, keys::Key, hotkey::{Hotkey, Hotkey::*}};
use windows::Win32::UI::Shell::IShellDispatch;
use windows::Win32::System::{Variant::VARIANT, Com::{CLSCTX_ALL, CoCreateInstance}};
use windows_core::{BSTR, GUID};

pub const NAME: &'_ str = "explorer";

pub fn open(path: impl AsRef<Path>) -> Result<(), Error> {
	let path: Vec<u16> = path
		.as_ref()
		.as_os_str()
		.encode_wide()
		.collect();
	
	Ok(unsafe { shell()?.Open(&VARIANT::from(BSTR::from_wide(&path)))? })
}

fn shell() -> Result<IShellDispatch, Error> {
	#[allow(non_upper_case_globals)]
	const CLSID_Shell: GUID = GUID::from_u128(0x13709620_C279_11CE_A49E_444553540000);
	unsafe { CoCreateInstance(&CLSID_Shell, None, CLSCTX_ALL).map_err(|err| err.into()) }
}

// hotkeys

pub fn focus_on_addr_bar() -> Hotkey { Remap(Mods::LA, Key::D) }
pub fn close_tab() -> Hotkey { Remap(Mods::LC, Key::W) }
pub fn next_tab() -> Hotkey { Remap(Mods::LC, Key::TAB) }
pub fn prev_tab() -> Hotkey { Remap(Mods::LCS, Key::TAB) }
pub fn new_tab() -> Hotkey { Remap(Mods::LC, Key::T) }
