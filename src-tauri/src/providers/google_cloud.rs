use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::Engine;
use std::time::Instant;

use super::{ProviderConfig, ProviderId, SttProvider, TranscriptionResult};
use crate::audio::encode_wav;

pub struct GoogleCloudProvider;

#[derive(serde::Serialize)]
struct GoogleRequest {
    config: GoogleConfig,
    audio: GoogleAudio,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleConfig {
    encoding: String,
    sample_rate_hertz: u32,
    language_code: String,
    model: String,
}

#[derive(serde::Serialize)]
struct GoogleAudio {
    content: String,
}

#[derive(serde::Deserialize)]
struct GoogleResponse {
    results: Option<Vec<GoogleResult>>,
}

#[derive(serde::Deserialize)]
struct GoogleResult {
    alternatives: Vec<GoogleAlternative>,
}

#[derive(serde::Deserialize)]
struct GoogleAlternative {
    transcript: String,
}

#[async_trait]
impl SttProvider for GoogleCloudProvider {
    fn id(&self) -> ProviderId {
        ProviderId::GoogleCloud
    }

    fn name(&self) -> &str {
        "Google Cloud STT"
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
            .ok_or_else(|| anyhow!("Google Cloud API key not configured"))?;

        let start = Instant::now();

        // Encode audio as WAV
        let wav_bytes = encode_wav(audio_data, 16000);
        let audio_content = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

        let language_code = config
            .language
            .as_deref()
            .unwrap_or("en-US")
            .to_string();

        let request = GoogleRequest {
            config: GoogleConfig {
                encoding: "LINEAR16".into(),
                sample_rate_hertz: 16000,
                language_code: language_code.clone(),
                model: config.model.as_deref().unwrap_or("default").into(),
            },
            audio: GoogleAudio {
                content: audio_content,
            },
        };

        let endpoint = config.endpoint.as_deref().unwrap_or(
            "https://speech.googleapis.com/v1/speech:recognize",
        );

        let url = format!("{}?key={}", endpoint, api_key);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Google Cloud API error ({}): {}",
                status,
                body
            ));
        }

        let result: GoogleResponse = response.json().await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        let text = result
            .results
            .and_then(|r| r.into_iter().next())
            .and_then(|r| r.alternatives.into_iter().next())
            .map(|a| a.transcript)
            .unwrap_or_default();

        Ok(TranscriptionResult {
            text,
            provider: ProviderId::GoogleCloud,
            duration_ms,
            language: Some(language_code),
        })
    }
}
