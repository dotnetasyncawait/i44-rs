pub mod mods;
pub mod keys;
pub mod hotkey;
pub mod hotstr;
pub mod key_event;

pub(super) mod handler;
mod extensions;
mod input_builder;
mod constants;

pub use macros::make_input as make;

pub struct InputKeys(pub Box<[InputKey]>, pub usize);
	
#[derive(Clone, Copy)]
pub enum InputKey {
	Unicode(u16, u16),
	Down(u16),
	Up(u16),
	Tap(u16, u16),
}