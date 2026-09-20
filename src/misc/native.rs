use windows::Win32::Foundation::HANDLE;
use std::num::NonZeroUsize;

pub(crate) struct NonNullHANDLE(NonZeroUsize);

impl NonNullHANDLE {
	pub fn as_handle(&self) -> HANDLE {
		HANDLE(self.0.get() as *mut std::ffi::c_void)
	}
	
	pub unsafe fn from_valid(h: HANDLE) -> Self {
		debug_assert!(!h.is_invalid());
		Self(unsafe { NonZeroUsize::new_unchecked(h.0 as usize) })
	}
}

impl windows_core::Free for NonNullHANDLE {
	unsafe fn free(&mut self) {
		unsafe { self.as_handle().free(); }
	}
}
