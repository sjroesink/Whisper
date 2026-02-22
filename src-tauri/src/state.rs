use std::sync::{Arc, Mutex};

use crate::audio::AudioRecorder;
use crate::history::TranscriptionHistory;
use crate::providers::ProviderManager;
use crate::settings::AppSettings;

pub struct AppState {
    pub recorder: Arc<Mutex<AudioRecorder>>,
    pub provider_manager: Arc<Mutex<ProviderManager>>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub history: Arc<Mutex<TranscriptionHistory>>,
    pub is_recording: Arc<Mutex<bool>>,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        let provider_manager = ProviderManager::new(&settings);
        Self {
            recorder: Arc::new(Mutex::new(AudioRecorder::new())),
            provider_manager: Arc::new(Mutex::new(provider_manager)),
            settings: Arc::new(Mutex::new(settings)),
            history: Arc::new(Mutex::new(TranscriptionHistory::new(100))),
            is_recording: Arc::new(Mutex::new(false)),
        }
    }
}
