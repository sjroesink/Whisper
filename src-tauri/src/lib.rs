mod audio;
mod clipboard;
mod commands;
mod history;
mod providers;
mod settings;
mod state;
mod tray;

use settings::{AppSettings, InteractionMode};
use state::AppState;
use tauri::{Emitter, Manager};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    let state = app.state::<AppState>();
                    let interaction_mode = {
                        let settings = state.settings.lock().unwrap();
                        settings.interaction_mode.clone()
                    };

                    match interaction_mode {
                        InteractionMode::PushToTalk => {
                            use tauri_plugin_global_shortcut::ShortcutState;
                            match event.state {
                                ShortcutState::Pressed => {
                                    handle_start_recording(app);
                                }
                                ShortcutState::Released => {
                                    handle_stop_recording(app);
                                }
                            }
                        }
                        InteractionMode::Toggle => {
                            use tauri_plugin_global_shortcut::ShortcutState;
                            if event.state == ShortcutState::Pressed {
                                let is_recording =
                                    *state.is_recording.lock().unwrap();
                                if is_recording {
                                    handle_stop_recording(app);
                                } else {
                                    handle_start_recording(app);
                                }
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Load settings
            let settings = AppSettings::load(app.handle());

            // Register global hotkey
            let hotkey = settings.hotkey.clone();

            // Initialize app state
            let app_state = AppState::new(settings);
            app.manage(app_state);

            // Setup system tray
            tray::setup_tray(app.handle())?;

            // Register the hotkey
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            if let Err(e) = app
                .handle()
                .global_shortcut()
                .on_shortcut(hotkey.as_str(), |_, _, _| {
                    // Handler is set in the plugin builder above
                })
            {
                log::error!("Failed to register hotkey '{}': {}", hotkey, e);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording_and_transcribe,
            commands::get_recording_state,
            commands::get_settings,
            commands::save_settings,
            commands::get_history,
            commands::clear_history,
            commands::get_providers,
        ])
        .run(tauri::generate_context!())
        .expect("error running whisper application");
}

fn handle_start_recording(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let mut recorder = state.recorder.lock().unwrap();
    if let Err(e) = recorder.start() {
        log::error!("Failed to start recording: {}", e);
        let _ = app.emit("error", format!("Failed to start recording: {}", e));
        return;
    }
    *state.is_recording.lock().unwrap() = true;
    let _ = app.emit("recording-started", ());
}

fn handle_stop_recording(app: &tauri::AppHandle) {
    let app_handle = app.clone();

    // Spawn async task for transcription
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<AppState>();

        // Stop recording
        let raw_audio = {
            let mut recorder = state.recorder.lock().unwrap();
            match recorder.stop() {
                Ok(audio) => audio,
                Err(e) => {
                    log::error!("Failed to stop recording: {}", e);
                    let _ = app_handle.emit("error", format!("Failed to stop recording: {}", e));
                    return;
                }
            }
        };
        *state.is_recording.lock().unwrap() = false;
        let _ = app_handle.emit("recording-stopped", ());

        if raw_audio.is_empty() {
            let _ = app_handle.emit("error", "No audio recorded".to_string());
            return;
        }

        let _ = app_handle.emit("transcribing", ());

        // Resample
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
        let result = provider.transcribe(&audio_16k, &config).await;

        match result {
            Ok(transcription) => {
                // Auto-paste
                {
                    let settings = state.settings.lock().unwrap();
                    if settings.auto_paste && !transcription.text.is_empty() {
                        if let Err(e) = crate::clipboard::paste_text(&transcription.text) {
                            log::error!("Auto-paste failed: {}", e);
                        }
                    }
                }

                // Add to history
                {
                    let mut history = state.history.lock().unwrap();
                    history.add(&transcription);
                }

                let _ = app_handle.emit("transcription-complete", &transcription);
            }
            Err(e) => {
                log::error!("Transcription failed: {}", e);
                let _ = app_handle.emit("error", format!("Transcription failed: {}", e));
            }
        }
    });
}
