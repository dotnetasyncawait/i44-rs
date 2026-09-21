#![allow(non_snake_case)]

use crate::common::error::{OsError, Win32ErrResExt, tagged_error};
use std::{ffi::c_void, fmt, io};
use std::{collections::HashMap, fs, io::Read, mem, path::{Path, PathBuf}, sync::{Arc, Mutex, mpsc}, thread};
use windows_core::{HRESULT, Interface};
use windows::Win32::Media::Audio::{
	AudioCategory_GameEffects, WAVEFORMATEX,
	XAudio2::{IXAudio2, IXAudio2MasteringVoice, IXAudio2SourceVoice, IXAudio2VoiceCallback, XAUDIO2_BUFFER,
		XAUDIO2_DEFAULT_CHANNELS, XAUDIO2_DEFAULT_FREQ_RATIO, XAUDIO2_DEFAULT_PROCESSOR, XAUDIO2_DEFAULT_SAMPLERATE,
		XAUDIO2_END_OF_STREAM}};

tagged_error!(
	PlayError,
	PlayErrorKind { UnsupportedFormat, Io, Os }
);

impl From<io::Error> for PlayError {
	fn from(value: io::Error) -> Self {
		Self::new_custom(PlayErrorKind::Io, value)
	}
}

impl From<OsError> for PlayError {
	fn from(value: OsError) -> Self {
		Self::new_custom(PlayErrorKind::Os, value)
	}
}

pub struct XAudio2 {
	audio: IXAudio2,
	cache: Mutex<HashMap<PathBuf, Arc<Chunks>>>,
	cb: ScopedVoiceCallback,
}

impl XAudio2 {
	pub fn new() -> Result<Self, OsError> {
		let mut audio: Option<IXAudio2> = None;
		unsafe { xaudio2_create(&mut audio, 0, XAUDIO2_DEFAULT_PROCESSOR)?; }
		
		let audio = audio.unwrap();
		let mut m_voice: Option<IXAudio2MasteringVoice> = None;
		
		unsafe { audio.CreateMasteringVoice(
			&mut m_voice,
			XAUDIO2_DEFAULT_CHANNELS,
			XAUDIO2_DEFAULT_SAMPLERATE,
			0, None, None, AudioCategory_GameEffects).context("failed to create mastering voice")?; }
		
		_ = m_voice;
		let (tx, rx) = mpsc::channel::<usize>();
		
		_ = thread::spawn(|| {
			for source in rx {
				unsafe { IXAudio2SourceVoice::from_raw(source as *mut c_void).DestroyVoice(); }
			}
		});
		
		Ok(Self { audio, cache: Mutex::new(HashMap::new()), cb: VoiceCallback::new(tx) })
	}
	
	pub fn play<P: AsRef<Path>>(&self, path: P) -> Result<(), PlayError> {
		self.play_inner(path, None)
	}
	
	pub fn play_vol<P: AsRef<Path>>(&self, path: P, vol: u8) -> Result<(), PlayError> {
		self.play_inner(path, Some(vol))
	}
	
	fn play_inner<P: AsRef<Path>>(&self, path: P, vol: Option<u8>) -> Result<(), PlayError> {
		let mut cache = self.cache.lock().unwrap();
		let key = path.as_ref();
		
		let chunks = match cache.get(key) {
			Some(ch) => Arc::clone(&ch),
			None => {
				drop(cache);
				let ch = Arc::new(parse(&path)?);
				let key = key.to_path_buf();
				cache = self.cache.lock().unwrap();
				_ = cache.insert(key, Arc::clone(&ch));
				ch
			}
		};
		drop(cache);
		
		let mut source: Option<IXAudio2SourceVoice> = None;
		
		unsafe { self.audio.CreateSourceVoice(
			&mut source,
			&chunks.fmt,
			0,
			XAUDIO2_DEFAULT_FREQ_RATIO,
			&self.cb.as_interface(), None, None).context("failed to create source voice")?; } 
		
		let source = source.unwrap();
		
		if let Some(mut vol) = vol {
			if vol > 100 { vol = 100; }
			unsafe { source.SetVolume(vol as f32 / 100f32, 0).context("failed to set volume")?; }
		}
		
		let buffer = XAUDIO2_BUFFER {
			Flags: XAUDIO2_END_OF_STREAM,
			AudioBytes: chunks.data.len() as u32,
			pAudioData: chunks.data.as_ptr(),
			pContext: source.as_raw(),
			..Default::default()
		};
		
		unsafe { source.SubmitSourceBuffer(&buffer, None).context("failed so submit source buffer")?; }
		unsafe { source.Start(0, 0).context("failed to start source voice")?; }
		
		Ok(())
	}
}

impl fmt::Debug for XAudio2 {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("XAudio2")
			// TODO
			.finish()
	}
}

unsafe impl Sync for XAudio2 {}
unsafe impl Send for XAudio2 {}

unsafe fn xaudio2_create(audio2: *mut Option<IXAudio2>, flags: u32, processor: u32) -> Result<(), OsError> {
	windows_core::link!("xaudio2_9.dll" "system"
		fn XAudio2Create(ppxaudio2: *mut *mut c_void, flags: u32, xaudio2processor: u32) -> HRESULT);
	
	unsafe { XAudio2Create(mem::transmute(audio2), flags, processor)
		.ok()
		.context("failed to create XAudio2") }
}

fn parse<P: AsRef<Path>>(path: P) -> Result<Chunks, PlayError> {
	let mut file = fs::File::open(&path)?;
	let t_size = file.metadata()?.len();
	
	// https://learn.microsoft.com/en-us/windows/win32/xaudio2/resource-interchange-file-format--riff-
	// RIFF + fSize + fType + ("fmt " + chSize + <data(16)> + "data" + chSize + <data(al_least_1_byte+padd)>).
	
	let mut riff = [0u8; 12];
	
	let is_valid = 
		file.read(&mut riff)? == riff.len() &&
		str::from_utf8(&riff[..4]).is_ok_and(|r| r == "RIFF") &&
		u32::from_le_bytes(riff[4..8].try_into().unwrap()) == (t_size - 8) as u32 &&
		str::from_utf8(&riff[8..12]).is_ok_and(|t| t == "WAVE");
	
	if !is_valid {
		return unsupported_format();
	}
	
	const PCM_LEN: usize = 16; // WAVE_FORMAT_PCM type length
	let mut fmt = &mut riff[..8];
	
	let is_valid =
		file.read(&mut fmt)? == fmt.len() && 
		str::from_utf8(&fmt[..4]).is_ok_and(|t| t == "fmt ") &&
		u32::from_le_bytes(fmt[4..8].try_into().unwrap()) == PCM_LEN as _;
	
	if !is_valid {
		return unsupported_format();
	}
	
	let mut wave_buff = [0u8; size_of::<WAVEFORMATEX>()];
	if file.read(&mut wave_buff[..PCM_LEN])? != PCM_LEN {
		return unsupported_format();
	}
	
	let wave: WAVEFORMATEX = unsafe { core::mem::transmute(wave_buff) };
	
	let mut data = fmt;
	if file.read(&mut data)? != data.len() || !str::from_utf8(&data[..4]).is_ok_and(|d| d == "data") {
		return unsupported_format();
	}
	
	let size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
	
	// TODO: is this sound?
	let mut data = unsafe { Box::<[u8]>::new_uninit_slice(size).assume_init() };
	
	if file.read(&mut data)? != size {
		unsupported_format()
	} else {
		Ok(Chunks { fmt: wave, data })
	}
}

fn unsupported_format<T>() -> Result<T, PlayError> {
	Err(PlayError::new_simple(PlayErrorKind::UnsupportedFormat))
}

struct Chunks {
	fmt: WAVEFORMATEX,
	data: Box<[u8]>,
}

impl fmt::Debug for Chunks {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Chunks")
			// TODO
			.finish()
	}
}


// https://github.com/microsoft/windows-rs/issues/4668
#[repr(C)]
struct VoiceCallback {
	vtbl: *const VoiceCallback_Vtbl,
	tx: mpsc::Sender<usize>,
}

impl VoiceCallback {
	fn new(tx: mpsc::Sender<usize>) -> ScopedVoiceCallback {
		let r_cb: *mut VoiceCallback = Box::into_raw(Box::new(Self { vtbl: &VoiceCallback_Vtbl::VTABLE, tx }));
		ScopedVoiceCallback(r_cb)
	}
}

struct ScopedVoiceCallback(*mut VoiceCallback);

impl ScopedVoiceCallback {
	fn as_interface(&self) -> IXAudio2VoiceCallback {
		unsafe { IXAudio2VoiceCallback::from_raw(self.0 as *mut c_void) }
	}
}

impl Drop for ScopedVoiceCallback {
	fn drop(&mut self) {
		_ = unsafe { Box::from_raw(self.0) };
	}
}

unsafe extern "system" fn OnBufferEnd(this: *mut c_void, ctx: *mut c_void) {
	let this = unsafe { &*(this as *const VoiceCallback) };
	this.tx.send(ctx as usize).expect("receiver should be alive");
}

unsafe extern "system" fn OnVoiceProcessingPassStart(_: *mut c_void, _: u32) {}
unsafe extern "system" fn OnVoiceProcessingPassEnd(_: *mut c_void) {}
unsafe extern "system" fn OnStreamEnd(_: *mut c_void) {}
unsafe extern "system" fn OnBufferStart(_: *mut c_void, _: *mut c_void) {}
unsafe extern "system" fn OnLoopEnd(_: *mut c_void, _: *mut c_void) {}
unsafe extern "system" fn OnVoiceError(_: *mut c_void, _: *mut c_void, _: HRESULT) {}

#[repr(C)]
struct VoiceCallback_Vtbl {
	OnVoiceProcessingPassStart: unsafe extern "system" fn(*mut c_void, u32),
	OnVoiceProcessingPassEnd: unsafe extern "system" fn(*mut c_void),
	OnStreamEnd: unsafe extern "system" fn(*mut c_void),
	OnBufferStart: unsafe extern "system" fn(*mut c_void, *mut c_void),
	OnBufferEnd: unsafe extern "system" fn(*mut c_void, *mut c_void),
	OnLoopEnd: unsafe extern "system" fn(*mut c_void, *mut c_void),
	OnVoiceError: unsafe extern "system" fn(*mut c_void, *mut c_void, HRESULT)
}

impl VoiceCallback_Vtbl {
	const VTABLE: Self = Self {
		OnVoiceProcessingPassStart, OnVoiceProcessingPassEnd, OnStreamEnd, OnBufferStart, OnBufferEnd, OnLoopEnd, OnVoiceError
	};
}