use tauri::{AppHandle, Emitter, State};

use crate::audio::AudioDevice;
use crate::history::TranscriptionEntry;
use crate::providers::ProviderInfo;
use crate::settings::AppSettings;
use crate::state::AppState;

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<AudioDevice>, String> {
    Ok(crate::audio::list_input_devices())
}

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let device_name = {
            let settings = state.settings.lock().map_err(|e| e.to_string())?;
            settings.input_device.clone()
        };
        let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
        recorder.start(&device_name).map_err(|e| e.to_string())?;
    }
    *state.is_recording.lock().unwrap() = true;
    let _ = app.emit("recording-started", ());
    Ok(())
}

#[tauri::command]
pub async fn stop_recording_and_transcribe(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Stop recording and get raw audio
    let raw_audio = {
        let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
        recorder.stop().map_err(|e| e.to_string())?
    };
    *state.is_recording.lock().unwrap() = false;
    let _ = app.emit("recording-stopped", ());

    if raw_audio.is_empty() {
        return Err("No audio recorded".into());
    }

    let _ = app.emit("transcribing", ());

    // Resample to 16kHz mono
    let audio_16k = {
        let recorder = state.recorder.lock().unwrap();
        recorder.get_audio_16khz_mono(raw_audio)
    };

    // Get provider and config (drop locks before await)
    let (provider, config) = {
        let pm = state.provider_manager.lock().unwrap();
        let settings = state.settings.lock().unwrap();
        let provider = pm.get_active(); // returns Arc, safe across await
        let config = settings.get_provider_config(&provider.id());
        (provider, config)
    };

    // Transcribe (no locks held)
    let result = provider
        .transcribe(&audio_16k, &config)
        .await
        .map_err(|e| e.to_string())?;

    // Auto-paste if enabled
    {
        let settings = state.settings.lock().unwrap();
        if settings.auto_paste && !result.text.is_empty() {
            if let Err(e) = crate::clipboard::paste_text(&result.text) {
                log::error!("Auto-paste failed: {}", e);
            }
        }
    }

    // Add to history
    {
        let mut history = state.history.lock().unwrap();
        history.add(&result);
    }

    let _ = app.emit("transcription-complete", &result);

    Ok(result.text)
}

#[tauri::command]
pub fn get_recording_state(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.is_recording.lock().unwrap())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    // Update provider manager's active provider
    {
        let mut pm = state.provider_manager.lock().unwrap();
        pm.set_active(settings.active_provider.clone());
    }

    // Persist settings
    settings.save(&app).map_err(|e| e.to_string())?;

    // Update in-memory settings
    {
        let mut current = state.settings.lock().unwrap();
        *current = settings;
    }

    Ok(())
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Result<Vec<TranscriptionEntry>, String> {
    Ok(state.history.lock().unwrap().get_all().to_vec())
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history.lock().unwrap().clear();
    Ok(())
}

#[tauri::command]
pub fn get_providers(state: State<'_, AppState>) -> Result<Vec<ProviderInfo>, String> {
    Ok(state.provider_manager.lock().unwrap().list_providers())
}
