use std::{path::Path, thread, os::windows::ffi::OsStrExt};
use std::sync::{OnceLock, mpsc::{self, Sender, Receiver}};
use crate::common::error::{OsError, Win32ErrResExt, tagged_error};
use crate::input::{mods::Mods, keys::Key, hotkey::{Hotkey, Hotkey::*}};
use crate::misc::win;
use regex::regex;
use windows_core::{BSTR, GUID, Interface, w};
use windows::Win32::{
	Foundation::HWND,
	System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, IServiceProvider},
	UI::Shell::{IShellBrowser, IShellDispatch, IShellFolderViewDual, IShellWindows, IWebBrowserApp, SWC_DESKTOP},
	UI::WindowsAndMessaging::{FindWindowExW, GetForegroundWindow},
	System::{Variant::VARIANT, Com::{CLSCTX_ALL, CoCreateInstance}},
};

pub const NAME: &'_ str = "explorer";

static WORKER: OnceLock<Sender<Job>> = OnceLock::new();

struct Job {
	hwnd: Option<HWND>, // TODO: use NonNullHWND
	tx: Sender<Result<Vec<String>, OsError>>,
}

impl Job {
	fn new(hwnd: Option<HWND>) -> (Self, Receiver<Result<Vec<String>, OsError>>) {
		let (tx, rx) = mpsc::channel();
		(Self { hwnd, tx }, rx)
	}
}

unsafe impl Send for Job {}

tagged_error!(
	Error,
	ErrorKind { NotInExplorer, WindowNotFound, Os }
);

impl From<OsError> for Error {
	fn from(value: OsError) -> Self {
		Self::new_custom(ErrorKind::Os, value)
	}
}

pub fn init() {
	let (tx, rx) = mpsc::channel::<Job>();
	let _ = thread::spawn(|| worker(rx));
	WORKER.set(tx).expect("WORKER should be initialized only once");
}

pub fn open(path: impl AsRef<Path>) -> Result<(), OsError> {
	let path: Vec<u16> = path
		.as_ref()
		.as_os_str()
		.encode_wide()
		.collect();
	
	Ok(unsafe { shell()?.Open(&VARIANT::from(BSTR::from_wide(&path)))? })
}

/// Returns paths of selected items in File Explorer.
/// # Errors
/// - [SelItemsErrorKind::WindowNotFound]: no foreground window
/// - [SelItemsErrorKind::NotInExplorer]: foreground window is not File Explorer
/// - [SelItemsErrorKind::Os]: system error
pub fn selected_items() -> Result<Vec<String>, Error> {
	let hwnd = unsafe { GetForegroundWindow() };
	
	let class_name = win::class_of(hwnd).map_err(|err| match err.kind() {
		win::ErrorKind::InvalidHwnd => Error::new_simple(ErrorKind::WindowNotFound),
		win::ErrorKind::Os => Error::new_custom(ErrorKind::Os, unsafe { err.into_os_unchecked() }),
		_ => unreachable!("win::class_of() returned unexpected error"),
	})?;
	
	let regex = regex!("^(?:(Progman|WorkerW)|(?:Cabinet|Explore)WClass)$");
	let Some(captures) = regex.captures(&class_name) else {
		return Err(Error::new_simple(ErrorKind::NotInExplorer));
	};
	
	let is_desktop = captures.get(1).is_some();
	let (job, promise) = Job::new((!is_desktop).then_some(hwnd));
	
	get_worker().send(job).unwrap();
	Ok(promise.recv().unwrap()?)
}

fn shell() -> Result<IShellDispatch, OsError> {
	#[allow(non_upper_case_globals)]
	const CLSID_Shell: GUID = GUID::from_u128(0x13709620_C279_11CE_A49E_444553540000);
	unsafe { CoCreateInstance(&CLSID_Shell, None, CLSCTX_ALL).context("failed to instantiate Shell") }
}

fn shell_windows() -> Result<IShellWindows, OsError> {
	#[allow(non_upper_case_globals)]
	const CLSID_ShellWindows: GUID = GUID::from_u128(0x9BA05972_F6A8_11CF_A442_00A0C90A8F39);
	unsafe { CoCreateInstance(&CLSID_ShellWindows, None, CLSCTX_ALL).context("failed to instantiate ShellWindows") }
}

fn get_worker() -> &'static Sender<Job> {
	WORKER.get().expect("WORKER should be initialized")
}

fn worker(rx: Receiver<Job>) {
	unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).unwrap(); }
	
	for job in rx {
		let response = match job.hwnd {
			Some(hwnd) => inner(hwnd),
			None => inner_desktop()
		};
		job.tx.send(response).unwrap();
	}
	
	fn inner(hwnd: HWND) -> Result<Vec<String>, OsError> {
		unsafe {
			let focused_tab = FindWindowExW(Some(hwnd), None, w!("ShellTabWindowClass"), None)
				.context("failed to find 'ShellTabWindowClass' control")?;
			
			let sh_windows = shell_windows()?;
			let count = sh_windows.Count().context("failed to get ShellWindows count")?;
			
			for i in 0..count {
				let item: IWebBrowserApp = sh_windows
					.Item(&i.into()).context("failed to get ShellWindows item")?
					.cast().context("failed to cast IWebBrowserApp")?;
				
				let item_hwnd = item.HWND().context("failed to get item HWND")?;
				if item_hwnd.0 != hwnd.0 as isize {
					continue;
				}
				
				let tab = item
					.cast::<IServiceProvider>().context("failed to query IServiceProvider")?
					.QueryService::<IShellBrowser>(&IShellBrowser::IID).context("failed to query IShellBrowser service")?
					.GetWindow().context("failed to get window")?;
				
				if tab == focused_tab {
					return get_selected_items(item);
				}
			}
			
			Ok(Vec::default())
		}
	}
	
	fn inner_desktop() -> Result<Vec<String>, OsError> {
		unsafe {
			let i: VARIANT = (SWC_DESKTOP.0 as u32).into();
			let item = shell_windows()?
				.Item(&i).context("failed to get Desktop item")?
				.cast::<IWebBrowserApp>().context("failed to query IWebBrowserApp")?;
			
			get_selected_items(item)
		}
	}
	
	fn get_selected_items(item: IWebBrowserApp) -> Result<Vec<String>, OsError> {
		unsafe {
			let selected_items = item
				.Document().context("failed to get item Document")?
				.cast::<IShellFolderViewDual>().context("failed to query IShellFolderViewDual")?
				.SelectedItems().context("failed to get selected items")?;
		
			let count: i32 = selected_items.Count().context("failed to get selected items' count")?;
			let mut paths = Vec::with_capacity(count as usize);
			
			for i in 0..count {
				let path = selected_items
					.Item(&i.into()).context("failed to get selected item")?
					.Path().context("failed to get selected item's path")
					.and_then(|p| Ok(String::from_utf16_lossy(&p)))?;
				
				paths.push(path);
			}
			Ok(paths)
		}
	}
}


// hotkeys

pub fn focus_on_addr_bar() -> Hotkey { Remap(Mods::LA, Key::D) }
pub fn close_tab() -> Hotkey { Remap(Mods::LC, Key::W) }
pub fn next_tab() -> Hotkey { Remap(Mods::LC, Key::TAB) }
pub fn prev_tab() -> Hotkey { Remap(Mods::LCS, Key::TAB) }
pub fn new_tab() -> Hotkey { Remap(Mods::LC, Key::T) }
