use std::{borrow::Cow, error::Error as StdError, fmt};
use windows_core::{Error as Win32Error, HRESULT};
use windows::Win32::Foundation::WIN32_ERROR;

pub const OK: Result<(), Error> = Ok(());

#[derive(Debug)]
pub struct Error {
	inner: Box<dyn StdError + 'static>,
}

impl<E: StdError + 'static> From<E> for Error {
	fn from(value: E) -> Self {
		Self { inner: Box::new(value) }
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.inner.fmt(f)
	}
}

#[derive(Debug)]
pub struct OsError {
	inner: Box<Inner>
}

#[derive(Debug)]
struct Inner {
	ctx: Cow<'static, str>,
	err: Win32Error,
}

impl OsError {
	pub fn new(ctx: impl Into<Cow<'static, str>>, err: Win32Error) -> Self {
		Self { inner: Box::new(Inner { ctx: ctx.into(), err }) }
	}
	
	pub fn from_hr(ctx: impl Into<Cow<'static, str>>, hr: HRESULT) -> Self {
		Self::new(ctx, Win32Error::from_hresult(hr))
	}
	
	pub fn from_win32(ctx: impl Into<Cow<'static, str>>, code: WIN32_ERROR) -> Self {
		Self::new(ctx, Win32Error::from(code))
	}
	
	pub fn from_thread(ctx: impl Into<Cow<'static, str>>) -> Self {
		Self::new(ctx, Win32Error::from_thread())
	}
	
	pub fn code(&self) -> HRESULT { self.inner.err.code() }
}

impl StdError for OsError {}

impl fmt::Display for OsError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.inner.ctx.is_empty() {
			self.inner.err.fmt(f)
		} else {
			write!(f, "{}: {}", self.inner.ctx, self.inner.err)
		}
	}
}

impl From<Win32Error> for OsError {
	fn from(value: Win32Error) -> Self {
		Self::new("", value)
	}
}

#[allow(dead_code)]
pub(crate) trait Win32ErrResExt {
	type OK;
	fn context(self, ctx: impl Into<Cow<'static, str>>) -> Result<Self::OK, OsError>;
	fn with_context<R: Into<Cow<'static, str>>>(self, ctx: impl FnOnce() -> R) -> Result<Self::OK, OsError>;
}

impl<T> Win32ErrResExt for Result<T, Win32Error> {
	type OK = T;
	
	fn context(self, ctx: impl Into<Cow<'static, str>>) -> Result<T, OsError> {
		self.map_err(|err| OsError::new(ctx, err))
	}
	
	fn with_context<C: Into<Cow<'static, str>>>(self, ctx: impl FnOnce() -> C) -> Result<T, OsError> {
		self.map_err(|err| OsError::new(ctx(), err))
	}
}

pub(crate) trait Win32ErrExt {
	fn as_win32(&self) -> WIN32_ERROR;
}

impl Win32ErrExt for Win32Error {
	fn as_win32(&self) -> WIN32_ERROR {
		WIN32_ERROR(self.code().0 as u32 & 0xFFFF)
	}
}

macro_rules! tagged_error {
	($Err:ident, $Kind:ident { $($Variant:ident),* $(,)? }) => {
		tagged_error!($Err, $Kind { $($Variant),* }, pub(super));
	};
	($Err:ident, $Kind:ident { $($Variant:ident),* $(,)? }, $internal:vis) => {
		const _: () = ::core::assert!(::core::mem::size_of::<::core::ptr::NonNull<()>>() == 8);
		
		::pastey::paste! { pub use [<__private_$Err:snake>]::$Err; }
		
		#[derive(::core::cmp::PartialEq, ::core::cmp::Eq, ::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug)]
		pub enum $Kind {
			$($Variant,)*
		}
		
		impl ::core::fmt::Display for $Kind {
			fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
				let str = match self {
					$(Self::$Variant => ::core::stringify!($Variant),)*
				};
				::core::fmt::Formatter::write_str(f, str)
			}
		}
		
		::pastey::paste! { mod [<__private_$Err:snake>] {
			use super::$Kind;
			
			const TAG_MASK: usize = 0b11;
			const TAG_CUSTOM: usize = 0b00;
			const TAG_SIMPLE: usize = 0b01;
			
			#[derive(::core::fmt::Debug)]
			pub struct $Err {
				repr: _Repr
			}
			
			#[allow(dead_code)]
			impl $Err {
				$internal fn new_simple(k: $Kind) -> Self {
					Self { repr: _Repr::new_simple(k) }
				}
				
				$internal fn new_custom<C>(k: $Kind, c: C) -> Self
				where
					C: ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static
				{
					Self { repr: _Repr::new_custom(k, c) }
				}
				
				pub fn kind(&self) -> $Kind {
					match self.repr.data() {
						_ErrorData::Simple(kind) => kind,
						_ErrorData::Custom(c) => c.kind,
					}
				}
				
				$internal fn extra<E>(&self) -> Option<&E>
				where
					E: ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static
				{
					match self.repr.data() {
						_ErrorData::Simple(_) => None,
						_ErrorData::Custom(c) => c.err_ref().downcast_ref(),
					}
				}
				
				$internal fn extra_unchecked<E>(&self) -> &E
				where
					E: ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static
				{
					let ptr = self.repr.0.as_ptr().wrapping_byte_sub(TAG_CUSTOM).cast::<_Custom>();
					unsafe { (&*ptr).err_ref().downcast_ref().unwrap_unchecked() }
				}
			
				$internal fn into_extra<E>(mut self) -> Result<E, Self>
				where
					E: ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static
				{
					let _ErrorData::Custom(c) = self.repr.data_mut() else {
						return Err(self);
					};
					
					let b_custom = unsafe { ::std::boxed::Box::from_raw(c) };
					let b_err = unsafe { ::std::boxed::Box::from_raw(b_custom.err.as_ptr()) };
					
					match b_err.downcast::<E>() {
						Ok(e) => {
							::core::mem::forget(*b_custom);
							::core::mem::forget(self);
							Ok(*e)
						}
						Err(err) => {
							let _ = ::std::boxed::Box::into_raw(b_custom);
							let _ = ::std::boxed::Box::into_raw(err);
							Err(self)
						}
					}
				}
				
				$internal unsafe fn into_extra_unchecked<E>(self) -> E
				where
					E: ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static
				{
					let ptr = self.repr.0.as_ptr().wrapping_byte_sub(TAG_CUSTOM).cast::<_Custom>();
					
					let b_custom = unsafe { ::std::boxed::Box::from_raw(&mut *ptr) };
					let b_err = unsafe { ::std::boxed::Box::from_raw(b_custom.err.as_ptr()) };
					
					let e = unsafe { b_err.downcast().unwrap_unchecked() };
					::core::mem::forget(*b_custom);
					::core::mem::forget(self);
					*e
				}
			}
			
			impl ::core::fmt::Display for $Err {
				fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
					match self.repr.data() {
						_ErrorData::Simple(kind) => ::core::write!(f, "{}({})", stringify!($Err), kind),
						_ErrorData::Custom(c) => ::core::write!(f, "{}({}: {})", stringify!($Err), c.kind, c.err_ref())
					}
				}
			}
			
			impl ::core::error::Error for $Err {}
			
			#[derive(::core::fmt::Debug)]
			#[repr(transparent)]
			struct _Repr(::core::ptr::NonNull<()>);
			
			impl _Repr {
				pub fn new_simple(k: $Kind) -> Self {
					let tagged = (k as usize) << 32 | TAG_SIMPLE;
					Self(unsafe { ::core::ptr::NonNull::without_provenance(::core::num::NonZeroUsize::new_unchecked(tagged)) })
				}
				
				pub fn new_custom<C>(k: $Kind, c: C) -> Self
				where
					C: ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static
				{
					let ptr = ::std::boxed::Box::into_raw(::std::boxed::Box::new(_Custom::new(k, c)));
					let tagged = ptr.wrapping_byte_add(TAG_CUSTOM).cast::<()>();
					Self(unsafe { ::core::ptr::NonNull::new_unchecked(tagged) })
				}
				
				pub fn data(&self) -> _ErrorData<&_Custom> {
					decode_repr(self.0, |c| unsafe { &*c })
				}
				
				pub fn data_mut(&mut self) -> _ErrorData<&mut _Custom> {
					decode_repr(self.0, |c| unsafe { &mut *c })
				}
			}
			
			fn decode_repr<C>(ptr: ::core::ptr::NonNull<()>, make_custom: fn(*mut _Custom) -> C) -> _ErrorData<C> {
				let bits = ptr.as_ptr().addr();
				
				match bits & TAG_MASK {
					TAG_CUSTOM => {
						let ptr = ptr.as_ptr().wrapping_byte_sub(TAG_CUSTOM).cast::<_Custom>();
						_ErrorData::Custom(make_custom(ptr))
					},
					TAG_SIMPLE => {
						let prim = (bits >> 32) as u32;
						_ErrorData::Simple(from_prim(prim))
					},
					_ => unsafe { ::core::hint::unreachable_unchecked(); }
				}
			}
			
			fn from_prim(prim: u32) -> $Kind {
				macro_rules! from_prim {
					($x:expr) => {
						match $x {
							$(v if v == $Kind::$Variant as _ => $Kind::$Variant,)*
							_ => unsafe { ::core::hint::unreachable_unchecked() }
						}
					}
				}
				from_prim!(prim)
			}
			
			impl ::core::ops::Drop for _Repr {
				fn drop(&mut self) {
					if let _ErrorData::Custom(c) = self.data_mut() {
						let _: ::std::boxed::Box<_Custom> = unsafe { ::std::boxed::Box::from_raw(c) };
					}
				}
			}
			
			struct _Custom {
				kind: $Kind,
				err: ::core::ptr::NonNull<dyn ::core::error::Error + ::core::marker::Send + ::core::marker::Sync>,
			}
			
			impl _Custom {
				fn new<E>(kind: $Kind, err: E) -> Self
				where
					E: ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static
				{
					let ptr = ::std::boxed::Box::into_raw(::std::boxed::Box::new(err));
					Self { kind, err: unsafe { ::core::ptr::NonNull::new_unchecked(ptr) } }
				}
				
				fn err_ref(&self) -> &(dyn ::core::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static) {
					unsafe { self.err.as_ref() }
				}
			}
			
			impl ::core::fmt::Display for _Custom {
				fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
					write!(f, "{}: {}", self.kind, self.err_ref())
				}
			}
			
			impl ::core::ops::Drop for _Custom {
				fn drop(&mut self) {
					let _ = unsafe { ::std::boxed::Box::from_raw(self.err.as_ptr()) };
				}
			}
			
			enum _ErrorData<C> {
				Simple($Kind),
				Custom(C)
			}
		}}
	}
}

pub(crate) use tagged_error;
