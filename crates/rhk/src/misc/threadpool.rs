use crate::common::error::{Error, OsError};
use std::{ffi::c_void, fmt};
use windows::Win32::System::Threading::{CloseThreadpoolWork, CreateThreadpoolWork, PTP_CALLBACK_INSTANCE, PTP_WORK,
	SubmitThreadpoolWork};

struct WorkItem {
	job: Box<dyn FnOnce() -> Result<(), Error>>,
}

pub fn queue_job(job: impl FnOnce() -> Result<(), Error> + Send + 'static) -> Result<(), OsError> {
	let item = Box::into_raw(Box::new(WorkItem { job: Box::new(job) }));
	
	match unsafe { CreateThreadpoolWork(Some(threadpool_proc), Some(item as *mut c_void), None) } {
		Ok(work) => {
			unsafe { SubmitThreadpoolWork(work); }
			Ok(())
		}
		Err(err) => {
			unsafe { _ = Box::from_raw(item); }
			Err(OsError::new("failed to create a threadpool work", err))
		}
	}
}

unsafe extern "system" fn threadpool_proc(_: PTP_CALLBACK_INSTANCE, ctx: *mut c_void, work: PTP_WORK) {
	let item = unsafe { Box::from_raw(ctx as *mut WorkItem) };
	
	if let Err(err) = (item.job)() {
		display_err("threadpool_proc", err);
	}
	
	unsafe { CloseThreadpoolWork(work); }
}

fn display_err(from: &str, err: impl fmt::Display) {
	// TODO: display with a window
	println!("{from}: {err}");
}
