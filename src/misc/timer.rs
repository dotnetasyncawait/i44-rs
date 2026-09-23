use crate::common::error::{Error, OsError, Win32ErrExt, Win32ErrResExt};
use std::{ffi::c_void, sync::OnceLock, fmt};
use windows::Win32::{
	Foundation::{HANDLE, ERROR_IO_PENDING},
	System::Threading::{CreateTimerQueue, CreateTimerQueueTimer, DeleteTimerQueueTimer, WT_EXECUTEONLYONCE},
};

static QUEUE: OnceLock<usize> = OnceLock::new();

pub fn init() {
	let queue = unsafe { CreateTimerQueue().expect("failed to create timer queue") };
	QUEUE.set(queue.0 as usize).expect("timer::init() should only be called once");
}

#[repr(C)]
struct TimerItem {
	timer: HANDLE,
	cb: Box<dyn FnOnce() -> Result<(), Error>>,
}

pub fn set_once(delay: u32, cb: impl FnOnce() -> Result<(), Error> + 'static) -> Result<(), OsError> {
	let item = TimerItem { timer: HANDLE::default(), cb: Box::new(cb) };
	let item = Box::into_raw(Box::new(item));
	
	if let Err(err) = unsafe { CreateTimerQueueTimer(
		item as *mut HANDLE,
		Some(get_queue()),
		Some(timer_proc),
		Some(item as *const c_void),
		delay,
		0,
		WT_EXECUTEONLYONCE).context("failed to create queue-timer") }
	{
		let _ = unsafe { Box::from_raw(item) };
		Err(err)
	} else {
		Ok(())
	}
}

unsafe extern "system" fn timer_proc(ptr: *mut c_void, _: bool) {
	let item = unsafe { Box::from_raw(ptr as *mut TimerItem) };
	
	if let Err(err) = (item.cb)() {
		display_err("timer_proc", err);
	}
	
	let timer = item.timer;
	
	// If the 'CompletionEvent' parameter is passed as NULL (None), the function marks the timer
	// for deletion and returns immediately. If the timer's procedure (us) is still running (which is),
	// it returns ERROR_IO_PENDING. Docs also say that it is not necessary to call this function again.
	// This approach should, technically, work without even needing a dedicated thread for timer deletion.
	if let Err(err) = unsafe {
		DeleteTimerQueueTimer(Some(get_queue()), timer, None) } && err.as_win32() != ERROR_IO_PENDING
	{
		panic!("failed to delete the timer ({timer:?}): {err}", );
	}
}

fn get_queue() -> HANDLE {
	HANDLE(*QUEUE.get().unwrap() as *mut c_void)
}

fn display_err(from: &str, err: impl fmt::Display) {
	// TODO: dispaly with a window
	println!("{from}: {err}");
}
