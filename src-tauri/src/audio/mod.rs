pub mod resampler;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

/// List all available audio input devices.
pub fn list_input_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok());

    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                let is_default = default_name.as_deref() == Some(&name);
                devices.push(AudioDevice { name, is_default });
            }
        }
    }
    devices
}

/// Find an input device by name, falling back to the default device.
fn find_device(device_name: &Option<String>) -> Result<cpal::Device> {
    let host = cpal::default_host();

    if let Some(name) = device_name {
        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if device.name().ok().as_deref() == Some(name.as_str()) {
                    return Ok(device);
                }
            }
        }
        log::warn!("Input device '{}' not found, falling back to default", name);
    }

    host.default_input_device()
        .ok_or_else(|| anyhow!("No input device found"))
}

pub struct AudioRecorder {
    stream: Option<Stream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

// cpal::Stream is not Send by default on all platforms but we manage it safely
unsafe impl Send for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host.default_input_device();

        let (sample_rate, channels) = device
            .as_ref()
            .and_then(|d| d.default_input_config().ok())
            .map(|cfg| (cfg.sample_rate().0, cfg.channels()))
            .unwrap_or((44100, 1));

        Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate,
            channels,
        }
    }

    pub fn start(&mut self, device_name: &Option<String>) -> Result<()> {
        let device = find_device(device_name)?;

        let config = device.default_input_config()?;
        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();

        // Clear existing buffer
        {
            let mut buf = self.buffer.lock().unwrap();
            buf.clear();
        }

        let buffer_clone = self.buffer.clone();
        let _channels = self.channels;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buf = buffer_clone.lock().unwrap();
                    buf.extend_from_slice(data);
                },
                |err| log::error!("Audio stream error: {}", err),
                None,
            )?,
            cpal::SampleFormat::I16 => {
                let buffer_clone = self.buffer.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let mut buf = buffer_clone.lock().unwrap();
                        for &sample in data {
                            buf.push(sample as f32 / i16::MAX as f32);
                        }
                    },
                    |err| log::error!("Audio stream error: {}", err),
                    None,
                )?
            }
            format => return Err(anyhow!("Unsupported sample format: {:?}", format)),
        };

        stream.play()?;
        self.stream = Some(stream);

        log::info!(
            "Recording started: {}Hz, {} channels",
            self.sample_rate,
            self.channels
        );
        Ok(())
    }

    pub fn stop(&mut self) -> Result<Vec<f32>> {
        // Drop the stream to stop recording
        self.stream.take();

        let raw_audio = {
            let mut buf = self.buffer.lock().unwrap();
            let data = buf.clone();
            buf.clear();
            data
        };

        log::info!("Recording stopped: {} samples captured", raw_audio.len());
        Ok(raw_audio)
    }

    pub fn get_audio_16khz_mono(&self, raw: Vec<f32>) -> Vec<f32> {
        resampler::resample_to_16khz_mono(&raw, self.sample_rate, self.channels)
    }

    #[allow(dead_code)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[allow(dead_code)]
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

/// Encode f32 samples as 16-bit PCM WAV bytes.
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let amplitude = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }
    writer.finalize().unwrap();
    cursor.into_inner()
}
