use crate::common::error::{Error, OsError};
use std::{ffi::c_void, fmt};
use windows::Win32::{
	Foundation::FILETIME,
	System::Threading::{CloseThreadpoolTimer, CreateThreadpoolTimer, IsThreadpoolTimerSet, PTP_CALLBACK_INSTANCE,
		PTP_TIMER, SetThreadpoolTimer, WaitForThreadpoolTimerCallbacks},
};

pub struct TimerGuard {
	timer: PTP_TIMER,
	item: *mut TimerItemResettable,
}

impl TimerGuard {
	pub fn reset(&self, delay: u32) {
		unsafe { SetThreadpoolTimer(self.timer, Some(&get_filetime(delay)), 0, None); }
	}
	
	pub fn is_set(&self) -> bool {
		unsafe { IsThreadpoolTimerSet(self.timer).as_bool() }
	}
	
	pub fn cancel(&self) {
		unsafe { SetThreadpoolTimer(self.timer, None, 0, None); }
		unsafe { WaitForThreadpoolTimerCallbacks(self.timer, true); }
	}
}

impl Drop for TimerGuard {
	fn drop(&mut self) {
		unsafe {
			SetThreadpoolTimer(self.timer, None, 0, None);
			WaitForThreadpoolTimerCallbacks(self.timer, true);
			CloseThreadpoolTimer(self.timer);
			
			// SAFETY: Timer is fully disposed, we can free the item now.
			_ = Box::from_raw(self.item);
		}
	}
}

unsafe impl Send for TimerGuard {}
unsafe impl Sync for TimerGuard {}

struct TimerItem {
	cb: Box<dyn FnOnce() -> Result<(), Error>>,
}

struct TimerItemResettable {
	cb: Box<dyn FnMut() -> Result<(), Error>>,
}

pub fn set_once(delay: u32, cb: impl FnOnce() -> Result<(), Error> + Send + 'static) -> Result<(), OsError> {
	let item = Box::into_raw(Box::new(TimerItem { cb: Box::new(cb) }));
	
	match unsafe { CreateThreadpoolTimer(Some(timer_proc_once), Some(item as *mut c_void), None) } {
		Ok(timer) => {
			unsafe { SetThreadpoolTimer(timer, Some(&get_filetime(delay)), 0, None); }
			Ok(())
		}
		Err(err) => {
			unsafe { _ = Box::from_raw(item); }
			Err(OsError::new("failed to create timer", err))
		}
	}
}

pub fn set_once_owned(
	delay: u32, cb: impl FnMut() -> Result<(), Error> + Send + 'static) -> Result<TimerGuard, OsError>
{
	let item = Box::into_raw(Box::new(TimerItemResettable { cb: Box::new(cb) }));
	
	match unsafe { CreateThreadpoolTimer(Some(timer_proc_resettable), Some(item as *mut c_void), None) } {
		Ok(timer) => {
			unsafe { SetThreadpoolTimer(timer, Some(&get_filetime(delay)), 0, None); }
			Ok(TimerGuard { timer, item })
		}
		Err(err) => {
			unsafe { _ = Box::from_raw(item); }
			Err(OsError::new("failed to create timer", err))
		}
	}
}

unsafe extern "system" fn timer_proc_once(_: PTP_CALLBACK_INSTANCE, ctx: *mut c_void, timer: PTP_TIMER) {
	let item = unsafe { Box::from_raw(ctx as *mut TimerItem) };
	
	if let Err(err) = (item.cb)() {
		display_err("timer_proc", err);
	}
	
	unsafe { CloseThreadpoolTimer(timer); }
}

unsafe extern "system" fn timer_proc_resettable(_: PTP_CALLBACK_INSTANCE, ctx: *mut c_void, _: PTP_TIMER) {
	// SAFETY: We never access this struct from outside until the timer is fully disposed,
	// ie, it's unset, pending callbacks are awaited, and itself is closed.
	let item = unsafe { &mut *(ctx as *mut TimerItemResettable) };
	
	if let Err(err) = (item.cb)() {
		display_err("timer_proc_resettable", err);
	}
}

fn get_filetime(delay: u32) -> FILETIME {
	let duetime = -((delay as i64) * 10_000) as u64;
	FILETIME { dwLowDateTime: duetime as u32, dwHighDateTime: (duetime >> 32) as u32 }
}

fn display_err(from: &str, err: impl fmt::Display) {
	// TODO: display with a window
	println!("{from}: {err}");
}
