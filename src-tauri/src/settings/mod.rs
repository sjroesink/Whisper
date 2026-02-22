use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::providers::{ProviderConfig, ProviderId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InteractionMode {
    PushToTalk,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub active_provider: ProviderId,
    pub interaction_mode: InteractionMode,
    pub hotkey: String,
    pub language: String,
    pub provider_configs: HashMap<ProviderId, ProviderConfig>,
    pub local_whisper_model_path: Option<String>,
    #[serde(default)]
    pub constme_whisper_dll_path: Option<String>,
    #[serde(default)]
    pub constme_whisper_model_path: Option<String>,
    #[serde(default)]
    pub constme_whisper_model_name: Option<String>,
    pub auto_paste: bool,
    pub show_overlay: bool,
    #[serde(default)]
    pub input_device: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_provider: ProviderId::OpenAiWhisper,
            interaction_mode: InteractionMode::Toggle,
            hotkey: "CommandOrControl+Shift+Space".into(),
            language: "auto".into(),
            provider_configs: HashMap::new(),
            local_whisper_model_path: None,
            constme_whisper_dll_path: None,
            constme_whisper_model_path: None,
            constme_whisper_model_name: None,
            auto_paste: true,
            show_overlay: true,
            input_device: None,
        }
    }
}

impl AppSettings {
    pub fn load(app: &AppHandle) -> Self {
        let store = app.store("settings.json");
        match store {
            Ok(store) => match store.get("app_settings") {
                Some(val) => serde_json::from_value(val.clone()).unwrap_or_default(),
                None => Self::default(),
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, app: &AppHandle) -> anyhow::Result<()> {
        let store = app.store("settings.json")?;
        store.set("app_settings", serde_json::to_value(self)?);
        Ok(())
    }

    pub fn get_provider_config(&self, id: &ProviderId) -> ProviderConfig {
        self.provider_configs
            .get(id)
            .cloned()
            .unwrap_or_default()
    }
}
