//! Const-me/Whisper provider - GPU-accelerated local speech-to-text using DirectCompute.
//!
//! This provider loads the native Whisper.dll (from github.com/Const-me/Whisper) and uses
//! its COM-style API to transcribe audio on the GPU via Direct3D 11 compute shaders.

pub mod download;
mod ffi;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use super::{ProviderConfig, ProviderId, SttProvider, TranscriptionResult};
use ffi::{ComModel, RustAudioBuffer, SFullParams, SModelSetup, SamplingStrategy, WhisperDll};

/// State that persists across transcriptions (DLL + model loaded once).
struct LoadedState {
    _dll: WhisperDll,
    model: ComModel,
}

// Safety: LoadedState is only accessed through a Mutex
unsafe impl Send for LoadedState {}

pub struct ConstmeWhisperProvider {
    loaded: Mutex<Option<LoadedState>>,
    dll_path: Mutex<Option<PathBuf>>,
    model_path: Mutex<Option<PathBuf>>,
}

impl ConstmeWhisperProvider {
    pub fn new(dll_path: Option<&str>, model_path: Option<&str>) -> Self {
        Self {
            loaded: Mutex::new(None),
            dll_path: Mutex::new(dll_path.map(PathBuf::from)),
            model_path: Mutex::new(model_path.map(PathBuf::from)),
        }
    }

    /// Update the configured paths (called when settings change).
    pub fn update_paths(&self, dll_path: Option<&str>, model_path: Option<&str>) {
        *self.dll_path.lock().unwrap() = dll_path.map(PathBuf::from);
        *self.model_path.lock().unwrap() = model_path.map(PathBuf::from);
        // Clear loaded state so it reloads with new paths
        *self.loaded.lock().unwrap() = None;
    }

    /// Ensure the DLL and model are loaded, loading them if necessary.
    fn ensure_loaded(&self) -> Result<()> {
        let mut loaded = self.loaded.lock().unwrap();
        if loaded.is_some() {
            return Ok(());
        }

        let dll_path = self.resolve_dll_path()?;
        let model_path = self.resolve_model_path()?;

        log::info!("Loading Whisper.dll from {:?}", dll_path);
        let dll = WhisperDll::load(&dll_path)?;

        let setup = SModelSetup::default();
        log::info!("Loading Whisper model from {:?}", model_path);
        let model = dll.load_model(&model_path, &setup)?;
        log::info!("Whisper model loaded successfully");

        *loaded = Some(LoadedState { _dll: dll, model });
        Ok(())
    }

    fn resolve_dll_path(&self) -> Result<PathBuf> {
        let configured = self.dll_path.lock().unwrap().clone();
        if let Some(path) = configured {
            if !path.as_os_str().is_empty() {
                return Ok(path);
            }
        }
        // Fall back to auto-download location
        download::dll_path()
    }

    fn resolve_model_path(&self) -> Result<PathBuf> {
        let configured = self.model_path.lock().unwrap().clone();
        if let Some(path) = configured {
            if !path.as_os_str().is_empty() {
                return Ok(path);
            }
        }
        // Fall back to auto-download default model location
        download::model_path("ggml-medium.bin")
    }
}

#[async_trait]
impl SttProvider for ConstmeWhisperProvider {
    fn id(&self) -> ProviderId {
        ProviderId::ConstmeWhisper
    }

    fn name(&self) -> &str {
        "Whisper GPU (DirectCompute)"
    }

    fn is_available(&self) -> bool {
        let dll_ok = self
            .resolve_dll_path()
            .map(|p| p.exists())
            .unwrap_or(false);
        let model_ok = self
            .resolve_model_path()
            .map(|p| p.exists())
            .unwrap_or(false);
        dll_ok && model_ok
    }

    async fn transcribe(
        &self,
        audio_data: &[f32],
        config: &ProviderConfig,
    ) -> Result<TranscriptionResult> {
        if audio_data.is_empty() {
            return Err(anyhow!("No audio data to transcribe"));
        }

        // Capture what we need before spawning blocking task
        let audio = audio_data.to_vec();
        let language = config.language.clone();

        // Ensure model is loaded (this is fast if already loaded)
        self.ensure_loaded()?;

        let start = Instant::now();

        // Get references we need for the blocking closure
        let loaded_guard = self.loaded.lock().unwrap();
        let state = loaded_guard
            .as_ref()
            .ok_or_else(|| anyhow!("Whisper model not loaded"))?;

        // Create a new context for this transcription
        let context = state.model.create_context()?;

        // Get default parameters
        let mut params = context.full_default_params(SamplingStrategy::Greedy)?;

        // Set language
        if let Some(lang) = &language {
            if lang != "auto" {
                params.language = SFullParams::make_language_key(lang);
            }
        }

        // Clear noisy flags (no progress printing, no timestamps in output)
        params.flags &= !(0x10 | 0x20 | 0x40); // Clear PrintProgress, PrintRealtime, PrintTimestamps

        // Create our Rust-implemented iAudioBuffer wrapping the audio data
        let audio_buffer_ptr = RustAudioBuffer::new(audio);
        let audio_buffer_void = RustAudioBuffer::as_ptr(audio_buffer_ptr);

        // Run transcription
        context.run_full(&params, audio_buffer_void)?;

        // Get results
        let result = context.get_results()?;
        let text = result.get_text()?;

        // The audio buffer will be released by the DLL calling Release on it
        // (it was passed as a COM pointer and the DLL manages the reference)

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text,
            provider: ProviderId::ConstmeWhisper,
            duration_ms,
            language: config.language.clone(),
        })
    }
}
