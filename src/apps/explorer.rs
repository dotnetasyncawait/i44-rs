use std::{path::Path, thread, os::windows::ffi::OsStrExt};
use std::sync::{OnceLock, mpsc::{self, Sender, Receiver}};
use crate::common::error::{ErrResultExt, Error, Win32ErrExt};
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
	hwnd: Option<HWND>,
	tx: Sender<Result<Vec<String>, Error>>,
}

impl Job {
	fn new(hwnd: Option<HWND>) -> (Self, Receiver<Result<Vec<String>, Error>>) {
		let (tx, rx) = mpsc::channel();
		(Self { hwnd, tx }, rx)
	}
}

unsafe impl Send for Job {}

pub fn init() {
	let (tx, rx) = mpsc::channel::<Job>();
	let _ = thread::spawn(|| worker(rx));
	WORKER.set(tx).expect("WORKER should be initialized only once");
}

pub fn open(path: impl AsRef<Path>) -> Result<(), Error> {
	let path: Vec<u16> = path
		.as_ref()
		.as_os_str()
		.encode_wide()
		.collect();
	
	Ok(unsafe { shell()?.Open(&VARIANT::from(BSTR::from_wide(&path)))? })
}

pub fn selected_items() -> Result<Vec<String>, Error> {
	let hwnd = unsafe { GetForegroundWindow() };
	if hwnd.is_invalid() {
		return Err(Error::other("no foreground window"));
	}
	
	let regex = regex!("^(?:(Progman|WorkerW)|(?:Cabinet|Explore)WClass)$");
	let class_name = win::class_of(hwnd);
	
	let Some(captures) = regex.captures(&class_name) else {
		return Err(Error::other("not in explorer"))
	};
	
	let is_desktop = captures.get(1).is_some();
	let (job, promise) = Job::new(if is_desktop { None } else { Some(hwnd) });
	
	get_worker().send(job).unwrap();
	promise.recv().unwrap()
}

fn shell() -> Result<IShellDispatch, Error> {
	#[allow(non_upper_case_globals)]
	const CLSID_Shell: GUID = GUID::from_u128(0x13709620_C279_11CE_A49E_444553540000);
	unsafe { CoCreateInstance(&CLSID_Shell, None, CLSCTX_ALL)
		.map_err(|err| err.with_context("failed to instantiate Shell").into()) }
}

fn shell_windows() -> Result<IShellWindows, Error> {
	#[allow(non_upper_case_globals)]
	const CLSID_ShellWindows: GUID = GUID::from_u128(0x9BA05972_F6A8_11CF_A442_00A0C90A8F39);
	unsafe { CoCreateInstance(&CLSID_ShellWindows, None, CLSCTX_ALL)
		.map_err(|err| err.with_context("failed to instantiate ShellWindows").into()) }
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
	
	fn inner(hwnd: HWND) -> Result<Vec<String>, Error> {
		unsafe {
			let focused_tab = FindWindowExW(Some(hwnd), None, w!("ShellTabWindowClass"), None)
				.with_context(|| "failed to find 'ShellTabWindowClass' control")?;
			
			let sh_windows = shell_windows()?;
			let count = sh_windows.Count().with_context(|| "failed to get ShellWindows count")?;
			
			for i in 0..count {
				let item: IWebBrowserApp = sh_windows
					.Item(&i.into()).with_context(|| "failed to get ShellWindows item")?
					.cast().unwrap();
				
				let item_hwnd = item.HWND().with_context(|| "failed to get item HWND")?;
				if item_hwnd.0 != hwnd.0 as isize {
					continue;
				}
				
				let tab = item
					.cast::<IServiceProvider>().unwrap()
					.QueryService::<IShellBrowser>(&IShellBrowser::IID).unwrap()
					.GetWindow().with_context(|| "failed to get window")?;
				
				if tab == focused_tab {
					return get_selected_items(item);
				}
			}
			
			Ok(vec![])
		}
	}
	
	fn inner_desktop() -> Result<Vec<String>, Error> {
		unsafe {
			let i = SWC_DESKTOP.0 as u32;
			let item = shell_windows()?
				.Item(&i.into()).with_context(|| "failed to get Desktop item")?
				.cast().unwrap();
			
			get_selected_items(item)
		}
	}
	
	fn get_selected_items(item: IWebBrowserApp) -> Result<Vec<String>, Error> {
		unsafe {
			let selected_items = item
				.Document().with_context(|| "failed to get item Document")?
				.cast::<IShellFolderViewDual>().unwrap()
				.SelectedItems().with_context(|| "failed to get selected items")?;
		
			let count: i32 = selected_items.Count().with_context(|| "failed to get selected items' count")?;
			let mut paths = Vec::with_capacity(count as usize);
			
			for i in 0..count {
				let path = selected_items
					.Item(&i.into()).with_context(|| "failed to get selected item")?
					.Path().with_context(|| "failed to get selected item's path")?
					.try_into().expect("path must be valid UTF-16");
				
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
