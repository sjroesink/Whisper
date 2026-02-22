//! FFI bindings for the Const-me/Whisper DLL (COM-style interface).
//!
//! The Const-me/Whisper library uses lightweight COM interfaces (ComLightLib).
//! Each interface starts with the standard IUnknown vtable (QueryInterface, AddRef, Release)
//! followed by interface-specific methods.

use std::ffi::c_void;
use std::os::raw::c_char;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{anyhow, Result};

/// HRESULT type matching Windows COM convention.
pub type HRESULT = i32;
pub const S_OK: HRESULT = 0;

/// GUID for QueryInterface (we don't actually need real GUIDs).
#[repr(C)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

// ---------------------------------------------------------------------------
// sModelSetup
// ---------------------------------------------------------------------------

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum ModelImplementation {
    GPU = 1,
    Hybrid = 2,
    Reference = 3,
}

#[repr(C)]
pub struct SModelSetup {
    pub implementation: ModelImplementation,
    pub flags: u32,
    pub adapter: *const u16, // const wchar_t* (null = default adapter)
}

impl Default for SModelSetup {
    fn default() -> Self {
        Self {
            implementation: ModelImplementation::GPU,
            flags: 0,
            adapter: std::ptr::null(),
        }
    }
}

// ---------------------------------------------------------------------------
// sLoadModelCallbacks
// ---------------------------------------------------------------------------

pub type PfnLoadProgress =
    Option<unsafe extern "system" fn(val: f64, pv: *mut c_void) -> HRESULT>;
pub type PfnCancel = Option<unsafe extern "system" fn(pv: *mut c_void) -> HRESULT>;

#[repr(C)]
pub struct SLoadModelCallbacks {
    pub progress: PfnLoadProgress,
    pub cancel: PfnCancel,
    pub pv: *mut c_void,
}

impl Default for SLoadModelCallbacks {
    fn default() -> Self {
        Self {
            progress: None,
            cancel: None,
            pv: std::ptr::null_mut(),
        }
    }
}

// ---------------------------------------------------------------------------
// sFullParams
// ---------------------------------------------------------------------------

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SamplingStrategy {
    Greedy = 0,
    BeamSearch = 1,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum FullParamsFlags {
    None = 0,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct SFullParams {
    pub strategy: SamplingStrategy,
    pub cpu_threads: i32,
    pub n_max_text_ctx: i32,
    pub offset_ms: i32,
    pub duration_ms: i32,
    pub flags: u32, // eFullParamsFlags bitmask
    pub language: u32,

    // Experimental timestamp params
    pub thold_pt: f32,
    pub thold_ptsum: f32,
    pub max_len: i32,
    pub max_tokens: i32,

    // Greedy params
    pub greedy_n_past: i32,

    // Beam search params
    pub beam_search_n_past: i32,
    pub beam_search_beam_width: i32,
    pub beam_search_n_best: i32,

    // Speed-up
    pub audio_ctx: i32,

    // Prompt tokens
    pub prompt_tokens: *const i32,
    pub prompt_n_tokens: i32,

    // Padding for alignment on x64
    _pad0: i32,

    // Callbacks (function pointers + user data)
    pub new_segment_callback: *const c_void,
    pub new_segment_callback_user_data: *mut c_void,
    pub encoder_begin_callback: *const c_void,
    pub encoder_begin_callback_user_data: *mut c_void,
}

impl SFullParams {
    /// Pack a language code string (e.g. "en", "nl") into a u32 key.
    pub fn make_language_key(code: &str) -> u32 {
        let bytes = code.as_bytes();
        let mut result: u32 = 0;
        for (i, &b) in bytes.iter().take(4).enumerate() {
            result |= (b as u32) << (i * 8);
        }
        result
    }
}

// Safety: SFullParams is only used within a single thread context
unsafe impl Send for SFullParams {}
unsafe impl Sync for SFullParams {}

// ---------------------------------------------------------------------------
// Result structures
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct STimeSpan {
    pub ticks: u64, // 100-nanosecond ticks
}

#[repr(C)]
pub struct STimeInterval {
    pub begin: STimeSpan,
    pub end: STimeSpan,
}

#[repr(C)]
pub struct SSegment {
    pub text: *const c_char,
    pub time: STimeInterval,
    pub first_token: u32,
    pub count_tokens: u32,
}

#[repr(C)]
pub struct STranscribeLength {
    pub count_segments: u32,
    pub count_tokens: u32,
}

/// eResultFlags::None - just get segments without tokens.
pub const RESULT_FLAGS_NONE: u32 = 0;

// ---------------------------------------------------------------------------
// COM Interface vtable definitions
// ---------------------------------------------------------------------------

/// Base COM IUnknown vtable methods.
type QueryInterfaceFn =
    unsafe extern "system" fn(this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT;
type AddRefFn = unsafe extern "system" fn(this: *mut c_void) -> u32;
type ReleaseFn = unsafe extern "system" fn(this: *mut c_void) -> u32;

// --- iModel vtable ---

#[repr(C)]
pub struct IModelVtbl {
    // IUnknown (3 methods)
    pub query_interface: QueryInterfaceFn,
    pub add_ref: AddRefFn,
    pub release: ReleaseFn,
    // iModel methods (6 methods)
    pub create_context:
        unsafe extern "system" fn(this: *mut c_void, pp: *mut *mut c_void) -> HRESULT,
    pub tokenize: unsafe extern "system" fn(
        this: *mut c_void,
        text: *const c_char,
        pfn: *const c_void,
        pv: *mut c_void,
    ) -> HRESULT,
    pub is_multilingual: unsafe extern "system" fn(this: *mut c_void) -> HRESULT,
    pub get_special_tokens:
        unsafe extern "system" fn(this: *mut c_void, rdi: *mut c_void) -> HRESULT,
    pub string_from_token:
        unsafe extern "system" fn(this: *mut c_void, token: u32) -> *const c_char,
    pub clone: unsafe extern "system" fn(this: *mut c_void, rdi: *mut *mut c_void) -> HRESULT,
}

/// Wrapper around a raw iModel COM pointer.
pub struct ComModel {
    ptr: *mut c_void,
}

impl ComModel {
    pub unsafe fn from_raw(ptr: *mut c_void) -> Self {
        Self { ptr }
    }

    fn vtbl(&self) -> &IModelVtbl {
        unsafe {
            let vtbl_ptr = *(self.ptr as *const *const IModelVtbl);
            &*vtbl_ptr
        }
    }

    pub fn create_context(&self) -> Result<ComContext> {
        let mut ctx_ptr: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { (self.vtbl().create_context)(self.ptr, &mut ctx_ptr) };
        if hr < 0 {
            return Err(anyhow!("iModel::createContext failed: HRESULT 0x{:08X}", hr as u32));
        }
        Ok(ComContext { ptr: ctx_ptr })
    }
}

impl Drop for ComModel {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                (self.vtbl().release)(self.ptr);
            }
        }
    }
}

// Safety: The COM object is thread-safe (the DLL handles synchronization)
unsafe impl Send for ComModel {}
unsafe impl Sync for ComModel {}

// --- iContext vtable ---

#[repr(C)]
pub struct IContextVtbl {
    // IUnknown (3 methods)
    pub query_interface: QueryInterfaceFn,
    pub add_ref: AddRefFn,
    pub release: ReleaseFn,
    // iContext methods (9 methods)
    pub run_full: unsafe extern "system" fn(
        this: *mut c_void,
        params: *const SFullParams,
        buffer: *mut c_void,
    ) -> HRESULT,
    pub run_streamed: unsafe extern "system" fn(
        this: *mut c_void,
        params: *const SFullParams,
        progress: *const c_void,
        reader: *mut c_void,
    ) -> HRESULT,
    pub run_capture: unsafe extern "system" fn(
        this: *mut c_void,
        params: *const SFullParams,
        callbacks: *const c_void,
        capture: *mut c_void,
    ) -> HRESULT,
    pub get_results: unsafe extern "system" fn(
        this: *mut c_void,
        flags: u32,
        pp: *mut *mut c_void,
    ) -> HRESULT,
    pub detect_speaker: unsafe extern "system" fn(
        this: *mut c_void,
        time: *const STimeInterval,
        result: *mut i32,
    ) -> HRESULT,
    pub get_model:
        unsafe extern "system" fn(this: *mut c_void, pp: *mut *mut c_void) -> HRESULT,
    pub full_default_params: unsafe extern "system" fn(
        this: *mut c_void,
        strategy: i32,
        rdi: *mut SFullParams,
    ) -> HRESULT,
    pub timings_print: unsafe extern "system" fn(this: *mut c_void) -> HRESULT,
    pub timings_reset: unsafe extern "system" fn(this: *mut c_void) -> HRESULT,
}

/// Wrapper around a raw iContext COM pointer.
pub struct ComContext {
    ptr: *mut c_void,
}

impl ComContext {
    fn vtbl(&self) -> &IContextVtbl {
        unsafe {
            let vtbl_ptr = *(self.ptr as *const *const IContextVtbl);
            &*vtbl_ptr
        }
    }

    /// Get default parameters for the given sampling strategy.
    pub fn full_default_params(&self, strategy: SamplingStrategy) -> Result<SFullParams> {
        let mut params: SFullParams = unsafe { std::mem::zeroed() };
        let hr = unsafe {
            (self.vtbl().full_default_params)(self.ptr, strategy as i32, &mut params)
        };
        if hr < 0 {
            return Err(anyhow!(
                "iContext::fullDefaultParams failed: HRESULT 0x{:08X}",
                hr as u32
            ));
        }
        Ok(params)
    }

    /// Run full transcription on an audio buffer.
    pub fn run_full(&self, params: &SFullParams, audio_buffer: *mut c_void) -> Result<()> {
        let hr = unsafe { (self.vtbl().run_full)(self.ptr, params, audio_buffer) };
        if hr < 0 {
            return Err(anyhow!(
                "iContext::runFull failed: HRESULT 0x{:08X}",
                hr as u32
            ));
        }
        Ok(())
    }

    /// Get transcription results.
    pub fn get_results(&self) -> Result<ComTranscribeResult> {
        let mut result_ptr: *mut c_void = std::ptr::null_mut();
        let hr = unsafe {
            (self.vtbl().get_results)(self.ptr, RESULT_FLAGS_NONE, &mut result_ptr)
        };
        if hr < 0 {
            return Err(anyhow!(
                "iContext::getResults failed: HRESULT 0x{:08X}",
                hr as u32
            ));
        }
        Ok(ComTranscribeResult { ptr: result_ptr })
    }
}

impl Drop for ComContext {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                (self.vtbl().release)(self.ptr);
            }
        }
    }
}

unsafe impl Send for ComContext {}

// --- iTranscribeResult vtable ---

#[repr(C)]
pub struct ITranscribeResultVtbl {
    // IUnknown (3 methods)
    pub query_interface: QueryInterfaceFn,
    pub add_ref: AddRefFn,
    pub release: ReleaseFn,
    // iTranscribeResult methods (3 methods)
    pub get_size: unsafe extern "system" fn(
        this: *mut c_void,
        rdi: *mut STranscribeLength,
    ) -> HRESULT,
    pub get_segments: unsafe extern "system" fn(this: *mut c_void) -> *const SSegment,
    pub get_tokens: unsafe extern "system" fn(this: *mut c_void) -> *const c_void,
}

/// Wrapper around a raw iTranscribeResult COM pointer.
pub struct ComTranscribeResult {
    ptr: *mut c_void,
}

impl ComTranscribeResult {
    fn vtbl(&self) -> &ITranscribeResultVtbl {
        unsafe {
            let vtbl_ptr = *(self.ptr as *const *const ITranscribeResultVtbl);
            &*vtbl_ptr
        }
    }

    /// Get the number of segments and tokens.
    pub fn get_size(&self) -> Result<STranscribeLength> {
        let mut len: STranscribeLength = STranscribeLength {
            count_segments: 0,
            count_tokens: 0,
        };
        let hr = unsafe { (self.vtbl().get_size)(self.ptr, &mut len) };
        if hr < 0 {
            return Err(anyhow!(
                "iTranscribeResult::getSize failed: HRESULT 0x{:08X}",
                hr as u32
            ));
        }
        Ok(len)
    }

    /// Get a pointer to the segments array.
    pub fn get_segments(&self) -> *const SSegment {
        unsafe { (self.vtbl().get_segments)(self.ptr) }
    }

    /// Read all segment texts and concatenate them.
    pub fn get_text(&self) -> Result<String> {
        let size = self.get_size()?;
        let segments = self.get_segments();
        if segments.is_null() || size.count_segments == 0 {
            return Ok(String::new());
        }

        let mut text = String::new();
        for i in 0..size.count_segments {
            let segment = unsafe { &*segments.add(i as usize) };
            if !segment.text.is_null() {
                let cstr = unsafe { std::ffi::CStr::from_ptr(segment.text) };
                if let Ok(s) = cstr.to_str() {
                    text.push_str(s);
                }
            }
        }
        Ok(text.trim().to_string())
    }
}

impl Drop for ComTranscribeResult {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                (self.vtbl().release)(self.ptr);
            }
        }
    }
}

unsafe impl Send for ComTranscribeResult {}

// ---------------------------------------------------------------------------
// iAudioBuffer - implemented in Rust (passed to C++ via COM interface)
// ---------------------------------------------------------------------------

/// Vtable for our Rust-implemented iAudioBuffer.
#[repr(C)]
pub struct IAudioBufferVtbl {
    pub query_interface: unsafe extern "system" fn(
        this: *mut RustAudioBuffer,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(this: *mut RustAudioBuffer) -> u32,
    pub release: unsafe extern "system" fn(this: *mut RustAudioBuffer) -> u32,
    pub count_samples: unsafe extern "system" fn(this: *const RustAudioBuffer) -> u32,
    pub get_pcm_mono: unsafe extern "system" fn(this: *const RustAudioBuffer) -> *const f32,
    pub get_pcm_stereo: unsafe extern "system" fn(this: *const RustAudioBuffer) -> *const f32,
    pub get_time:
        unsafe extern "system" fn(this: *const RustAudioBuffer, rdi: *mut i64) -> HRESULT,
}

/// Our Rust implementation of iAudioBuffer COM interface.
/// The first field MUST be the vtable pointer (standard COM layout).
#[repr(C)]
pub struct RustAudioBuffer {
    vtbl: *const IAudioBufferVtbl,
    ref_count: AtomicU32,
    samples: Vec<f32>,
    sample_count: u32,
}

// Static vtable instance
static AUDIO_BUFFER_VTBL: IAudioBufferVtbl = IAudioBufferVtbl {
    query_interface: audio_buffer_query_interface,
    add_ref: audio_buffer_add_ref,
    release: audio_buffer_release,
    count_samples: audio_buffer_count_samples,
    get_pcm_mono: audio_buffer_get_pcm_mono,
    get_pcm_stereo: audio_buffer_get_pcm_stereo,
    get_time: audio_buffer_get_time,
};

impl RustAudioBuffer {
    /// Create a new audio buffer wrapping the given PCM mono samples (expected 16kHz).
    /// Returns a raw pointer suitable for passing as iAudioBuffer* to the DLL.
    pub fn new(samples: Vec<f32>) -> *mut Self {
        let sample_count = samples.len() as u32;
        let buffer = Box::new(Self {
            vtbl: &AUDIO_BUFFER_VTBL,
            ref_count: AtomicU32::new(1),
            samples,
            sample_count,
        });
        Box::into_raw(buffer)
    }

    /// Get this buffer as a void pointer for FFI.
    pub fn as_ptr(ptr: *mut Self) -> *mut c_void {
        ptr as *mut c_void
    }
}

// COM method implementations for RustAudioBuffer

unsafe extern "system" fn audio_buffer_query_interface(
    _this: *mut RustAudioBuffer,
    _riid: *const GUID,
    _ppv: *mut *mut c_void,
) -> HRESULT {
    // We don't support QueryInterface; the caller already has the right pointer
    -2147467262_i32 // E_NOINTERFACE
}

unsafe extern "system" fn audio_buffer_add_ref(this: *mut RustAudioBuffer) -> u32 {
    let obj = &*this;
    obj.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn audio_buffer_release(this: *mut RustAudioBuffer) -> u32 {
    let obj = &*this;
    let prev = obj.ref_count.fetch_sub(1, Ordering::Release);
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(this));
        return 0;
    }
    prev - 1
}

unsafe extern "system" fn audio_buffer_count_samples(this: *const RustAudioBuffer) -> u32 {
    let obj = &*this;
    obj.sample_count
}

unsafe extern "system" fn audio_buffer_get_pcm_mono(this: *const RustAudioBuffer) -> *const f32 {
    let obj = &*this;
    obj.samples.as_ptr()
}

unsafe extern "system" fn audio_buffer_get_pcm_stereo(_this: *const RustAudioBuffer) -> *const f32 {
    // We only provide mono audio
    std::ptr::null()
}

unsafe extern "system" fn audio_buffer_get_time(
    _this: *const RustAudioBuffer,
    rdi: *mut i64,
) -> HRESULT {
    if !rdi.is_null() {
        *rdi = 0;
    }
    S_OK
}

// ---------------------------------------------------------------------------
// DLL loading
// ---------------------------------------------------------------------------

/// Type for the loadModel DLL export.
type LoadModelFn = unsafe extern "system" fn(
    path: *const u16,               // const wchar_t*
    setup: *const SModelSetup,      // const sModelSetup&
    callbacks: *const SLoadModelCallbacks, // const sLoadModelCallbacks*
    pp: *mut *mut c_void,           // iModel**
) -> HRESULT;

/// Loaded Whisper DLL with the model factory function.
pub struct WhisperDll {
    _library: libloading::Library,
    load_model: LoadModelFn,
}

impl WhisperDll {
    /// Load the Whisper.dll from the given path.
    pub fn load(dll_path: &Path) -> Result<Self> {
        if !dll_path.exists() {
            return Err(anyhow!("Whisper.dll not found at {:?}", dll_path));
        }

        let library = unsafe { libloading::Library::new(dll_path) }
            .map_err(|e| anyhow!("Failed to load Whisper.dll: {}", e))?;

        // Try to find the loadModel export - it might be exported with different names
        let load_model = unsafe {
            // Try C-style export name first
            let func: Result<libloading::Symbol<LoadModelFn>, _> = library.get(b"loadModel");
            if let Ok(f) = func {
                *f
            } else {
                // Try with Whisper namespace mangling patterns
                let func: libloading::Symbol<LoadModelFn> = library
                    .get(b"?loadModel@Whisper@@YAJPEBGAEBUsModelSetup@1@PEBUsLoadModelCallbacks@1@PEAPEAUiModel@1@@Z")
                    .map_err(|e| anyhow!("Failed to find loadModel export in Whisper.dll: {}", e))?;
                *func
            }
        };

        Ok(Self {
            _library: library,
            load_model,
        })
    }

    /// Load a Whisper model from the given GGML file path.
    pub fn load_model(&self, model_path: &Path, setup: &SModelSetup) -> Result<ComModel> {
        if !model_path.exists() {
            return Err(anyhow!("Model file not found at {:?}", model_path));
        }

        // Convert path to wide string (wchar_t*)
        let wide_path: Vec<u16> = model_path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut model_ptr: *mut c_void = std::ptr::null_mut();
        let hr = unsafe {
            (self.load_model)(
                wide_path.as_ptr(),
                setup,
                std::ptr::null(),
                &mut model_ptr,
            )
        };

        if hr < 0 {
            return Err(anyhow!(
                "Failed to load Whisper model: HRESULT 0x{:08X}",
                hr as u32
            ));
        }

        Ok(unsafe { ComModel::from_raw(model_ptr) })
    }
}

// Safety: The DLL and its function pointers are safe to send between threads
unsafe impl Send for WhisperDll {}
unsafe impl Sync for WhisperDll {}
