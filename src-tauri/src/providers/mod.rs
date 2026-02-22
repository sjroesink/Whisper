pub mod constme_whisper;
pub mod google_cloud;
pub mod local_whisper;
pub mod native_stt;
pub mod openai_whisper;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::settings::AppSettings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProviderId {
    OpenAiWhisper,
    GoogleCloud,
    LocalWhisper,
    NativeStt,
    ConstmeWhisper,
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderId::OpenAiWhisper => write!(f, "OpenAI Whisper"),
            ProviderId::GoogleCloud => write!(f, "Google Cloud"),
            ProviderId::LocalWhisper => write!(f, "Local Whisper"),
            ProviderId::NativeStt => write!(f, "Native STT"),
            ProviderId::ConstmeWhisper => write!(f, "Whisper GPU (DirectCompute)"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub language: Option<String>,
    pub endpoint: Option<String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: None,
            language: Some("auto".into()),
            endpoint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub provider: ProviderId,
    pub duration_ms: u64,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub name: String,
    pub available: bool,
}

#[async_trait]
pub trait SttProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    async fn transcribe(
        &self,
        audio_data: &[f32],
        config: &ProviderConfig,
    ) -> Result<TranscriptionResult>;
}

pub struct ProviderManager {
    providers: Vec<std::sync::Arc<dyn SttProvider>>,
    active_provider: ProviderId,
}

impl ProviderManager {
    pub fn new(settings: &AppSettings) -> Self {
        let providers: Vec<std::sync::Arc<dyn SttProvider>> = vec![
            std::sync::Arc::new(openai_whisper::OpenAiWhisperProvider),
            std::sync::Arc::new(google_cloud::GoogleCloudProvider),
            std::sync::Arc::new(local_whisper::LocalWhisperProvider::new(
                settings.local_whisper_model_path.as_deref(),
            )),
            std::sync::Arc::new(native_stt::NativeSttProvider),
            std::sync::Arc::new(constme_whisper::ConstmeWhisperProvider::new(
                settings.constme_whisper_dll_path.as_deref(),
                settings.constme_whisper_model_path.as_deref(),
            )),
        ];

        Self {
            providers,
            active_provider: settings.active_provider.clone(),
        }
    }

    pub fn set_active(&mut self, id: ProviderId) {
        self.active_provider = id;
    }

    /// Returns an Arc clone of the active provider (safe to use across await points).
    pub fn get_active(&self) -> std::sync::Arc<dyn SttProvider> {
        self.providers
            .iter()
            .find(|p| p.id() == self.active_provider)
            .cloned()
            .unwrap_or_else(|| self.providers[0].clone())
    }

    pub fn list_providers(&self) -> Vec<ProviderInfo> {
        self.providers
            .iter()
            .map(|p| ProviderInfo {
                id: p.id(),
                name: p.name().to_string(),
                available: p.is_available(),
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn active_id(&self) -> &ProviderId {
        &self.active_provider
    }
}
