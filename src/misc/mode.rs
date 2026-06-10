use std::{thread, sync::{OnceLock, RwLock}};
use crate::common::kb::{I44, layers::*, hid_msgs::*};

#[derive(Debug, Clone, Copy)]
pub enum ModeState {
	Normal,
	Insert,
	Select,
	NSymbol, // Normal + Symbol
	ISymbol, // Insert + Symbol
	SSymbol, // Select + Symbol
	USymbol, // Upper Symbol
	Mouse,
	System,
	None
}

static MODE: OnceLock<RwLock<Mode>> = OnceLock::new();

#[derive(Debug)]
pub struct Mode {
	state: ModeState
}

impl Mode {
	pub fn get() -> ModeState {
		match MODE.get() {
			Some(mode) => mode.read().unwrap().state,
			None => ModeState::None
		}
	}
}

pub(crate) fn start() {
	MODE
		.set(RwLock::new(Mode{ state: ModeState::None }))
		.expect("mode should only be initialized once");
	
	let _ = thread::spawn(hid_listener);
}

pub(crate) fn exit() {
	// TODO: how to stop it?
}

fn hid_listener() {
	let mut d = I44::new_device();
	
	d.open().and_then(|_| d.write(&[HID_GET_LAYER])).unwrap();
	let mut input = [0u8; 3];
	
	loop {
		d.read(&mut input).unwrap();
		
		if input[0] == HID_LAYER_UPDATE {
			let state = (input[1] as u16) << 8 | input[2] as u16;
			set_state(state);
		}
	}
}

fn set_state(layer: u16) {
	let state = match bit_on_16(layer) {
		NORMAL => ModeState::Normal,                      // 0000_000{0|1}
		INSERT => ModeState::Insert,                      // 0000_0010
		SELECT => ModeState::Select,                      // 0000_0100
		SYMBOL if layer & 0x2 != 0 => ModeState::ISymbol, // 0000_1010
		SYMBOL if layer & 0x4 != 0 => ModeState::SSymbol, // 0000_1100
		SYMBOL => ModeState::NSymbol,                     // 0000_1000
		U_SYMB => ModeState::USymbol,                     // 0001_0000
		MOUSE  => ModeState::Mouse,                       // 0010_0000
		SYSTEM => ModeState::System,                      // 0100_0000
		_ => unreachable!("unexpected layer-state: 0b{layer:b}")
	};
	
	MODE.get().unwrap().write().unwrap().state = state;
	
	println!("state update: {state:?}");
	// TODO: update window
}

fn bit_on_16(mut bits: u16) -> u8 {
	let mut n = 0;
	
	if bits >> 8 != 0 {
		n += 8;
		bits >>= 8;
	}
	if bits >> 4 != 0 {
		n += 4;
		bits >>= 4;
	}
	if bits >> 2 != 0 {
		n += 2;
		bits >>= 2;
	}
	if bits >> 1 != 0 {
		n += 1;
	}
	
	n
}