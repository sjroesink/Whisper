use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::path::PathBuf;

use super::{ProviderConfig, ProviderId, SttProvider, TranscriptionResult};

pub struct LocalWhisperProvider {
    #[allow(dead_code)]
    model_path: Option<PathBuf>,
}

impl LocalWhisperProvider {
    pub fn new(model_path: Option<&str>) -> Self {
        Self {
            model_path: model_path.map(PathBuf::from),
        }
    }
}

#[async_trait]
impl SttProvider for LocalWhisperProvider {
    fn id(&self) -> ProviderId {
        ProviderId::LocalWhisper
    }

    fn name(&self) -> &str {
        "Local Whisper"
    }

    fn is_available(&self) -> bool {
        #[cfg(feature = "local-whisper")]
        {
            return self.model_path.as_ref().map_or(false, |p| p.exists());
        }
        #[cfg(not(feature = "local-whisper"))]
        {
            false
        }
    }

    async fn transcribe(
        &self,
        _audio_data: &[f32],
        _config: &ProviderConfig,
    ) -> Result<TranscriptionResult> {
        #[cfg(feature = "local-whisper")]
        {
            return transcribe_local(audio_data, config, &self.model_path).await;
        }
        #[cfg(not(feature = "local-whisper"))]
        {
            Err(anyhow!(
                "Local Whisper not enabled. Rebuild with --features local-whisper (requires LLVM/clang)."
            ))
        }
    }
}

#[cfg(feature = "local-whisper")]
async fn transcribe_local(
    audio_data: &[f32],
    config: &ProviderConfig,
    model_path: &Option<PathBuf>,
) -> Result<TranscriptionResult> {
    use std::time::Instant;

    let model_path = model_path
        .clone()
        .ok_or_else(|| anyhow!("No whisper model path configured"))?;

    if !model_path.exists() {
        return Err(anyhow!("Whisper model not found at {:?}", model_path));
    }

    let audio = audio_data.to_vec();
    let language = config.language.clone();
    let start = Instant::now();

    let text = tokio::task::spawn_blocking(move || -> Result<String> {
        let ctx = whisper_rs::WhisperContext::new_with_params(
            model_path.to_str().unwrap(),
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow!("Failed to load whisper model: {}", e))?;

        let mut state = ctx
            .create_state()
            .map_err(|e| anyhow!("Failed to create whisper state: {}", e))?;

        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });

        if let Some(lang) = &language {
            if lang != "auto" {
                params.set_language(Some(lang));
            }
        }
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, &audio)
            .map_err(|e| anyhow!("Whisper transcription failed: {}", e))?;

        let mut text = String::new();
        let num_segments = state
            .full_n_segments()
            .map_err(|e| anyhow!("Failed to get segments: {}", e))?;

        for i in 0..num_segments {
            let segment = state
                .full_get_segment_text(i)
                .map_err(|e| anyhow!("Failed to get segment text: {}", e))?;
            text.push_str(&segment);
        }

        Ok(text.trim().to_string())
    })
    .await??;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(TranscriptionResult {
        text,
        provider: ProviderId::LocalWhisper,
        duration_ms,
        language: config.language.clone(),
    })
}
