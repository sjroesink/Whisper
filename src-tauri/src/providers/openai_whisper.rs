use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::multipart;
use std::time::Instant;

use super::{ProviderConfig, ProviderId, SttProvider, TranscriptionResult};
use crate::audio::encode_wav;

pub struct OpenAiWhisperProvider;

#[derive(serde::Deserialize)]
struct WhisperResponse {
    text: String,
}

#[async_trait]
impl SttProvider for OpenAiWhisperProvider {
    fn id(&self) -> ProviderId {
        ProviderId::OpenAiWhisper
    }

    fn name(&self) -> &str {
        "OpenAI Whisper"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn transcribe(
        &self,
        audio_data: &[f32],
        config: &ProviderConfig,
    ) -> Result<TranscriptionResult> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

        let start = Instant::now();

        // Encode audio as WAV
        let wav_bytes = encode_wav(audio_data, 16000);

        // Build multipart form
        let file_part = multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        let model = config
            .model
            .as_deref()
            .unwrap_or("whisper-1")
            .to_string();

        let mut form = multipart::Form::new()
            .part("file", file_part)
            .text("model", model);

        if let Some(lang) = &config.language {
            if lang != "auto" {
                form = form.text("language", lang.clone());
            }
        }

        let endpoint = config
            .endpoint
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/audio/transcriptions");

        let client = reqwest::Client::new();
        let response = client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "OpenAI API error ({}): {}",
                status,
                body
            ));
        }

        let result: WhisperResponse = response.json().await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text: result.text,
            provider: ProviderId::OpenAiWhisper,
            duration_ms,
            language: config.language.clone(),
        })
    }
}
