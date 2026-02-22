use anyhow::Result;
use async_trait::async_trait;
use std::time::Instant;

use super::{ProviderConfig, ProviderId, SttProvider, TranscriptionResult};

pub struct NativeSttProvider;

#[async_trait]
impl SttProvider for NativeSttProvider {
    fn id(&self) -> ProviderId {
        ProviderId::NativeStt
    }

    fn name(&self) -> &str {
        "OS Native STT"
    }

    fn is_available(&self) -> bool {
        cfg!(any(target_os = "windows", target_os = "macos"))
    }

    async fn transcribe(
        &self,
        audio_data: &[f32],
        config: &ProviderConfig,
    ) -> Result<TranscriptionResult> {
        let start = Instant::now();

        #[cfg(target_os = "windows")]
        let text = transcribe_windows(audio_data, config).await?;

        #[cfg(target_os = "macos")]
        let text = transcribe_macos(audio_data, config).await?;

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let text = return Err(anyhow!("Native STT not available on this platform"));

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text,
            provider: ProviderId::NativeStt,
            duration_ms,
            language: config.language.clone(),
        })
    }
}

#[cfg(target_os = "windows")]
async fn transcribe_windows(audio_data: &[f32], config: &ProviderConfig) -> Result<String> {
    use crate::audio::encode_wav;

    let audio = audio_data.to_vec();
    let _language = config.language.clone();

    tokio::task::spawn_blocking(move || -> Result<String> {
        let wav_bytes = encode_wav(&audio, 16000);

        // Write WAV to a temporary file for SAPI to consume
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("whisper_native_temp.wav");
        std::fs::write(&temp_path, &wav_bytes)?;

        // Use Windows SAPI via COM
        // This is a simplified implementation using command-line fallback
        // Full SAPI COM integration would use the `windows` crate directly
        let output = std::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    r#"
                    Add-Type -AssemblyName System.Speech
                    $recognizer = New-Object System.Speech.Recognition.SpeechRecognitionEngine
                    $recognizer.SetInputToWaveFile("{}")
                    $recognizer.LoadGrammar((New-Object System.Speech.Recognition.DictationGrammar))
                    try {{
                        $result = $recognizer.Recognize()
                        if ($result) {{ $result.Text }} else {{ "" }}
                    }} catch {{ "" }}
                    $recognizer.Dispose()
                    "#,
                    temp_path.to_string_lossy().replace('\\', "\\\\")
                ),
            ])
            .output()?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(text)
    })
    .await?
}

#[cfg(target_os = "macos")]
async fn transcribe_macos(audio_data: &[f32], config: &ProviderConfig) -> Result<String> {
    let audio = audio_data.to_vec();
    let language = config.language.clone();

    tokio::task::spawn_blocking(move || -> Result<String> {
        let wav_bytes = encode_wav(&audio, 16000);

        // Write WAV to a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("whisper_native_temp.wav");
        std::fs::write(&temp_path, &wav_bytes)?;

        // Use macOS Speech framework via swift command-line bridge
        let lang_arg = language
            .as_deref()
            .unwrap_or("en-US")
            .to_string();

        let output = std::process::Command::new("swift")
            .args([
                "-e",
                &format!(
                    r#"
                    import Speech
                    import Foundation

                    let semaphore = DispatchSemaphore(value: 0)
                    var resultText = ""

                    let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "{}"))
                    let url = URL(fileURLWithPath: "{}")
                    let request = SFSpeechURLRecognitionRequest(url: url)

                    recognizer?.recognitionTask(with: request) {{ result, error in
                        if let result = result, result.isFinal {{
                            resultText = result.bestTranscription.formattedString
                        }}
                        semaphore.signal()
                    }}

                    semaphore.wait()
                    print(resultText)
                    "#,
                    lang_arg,
                    temp_path.to_string_lossy()
                ),
            ])
            .output()?;

        let _ = std::fs::remove_file(&temp_path);

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(text)
    })
    .await?
}
