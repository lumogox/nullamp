use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

/// Target sample rate for Whisper (16kHz mono).
const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Captures microphone audio and resamples to 16kHz mono f32 for Whisper.
pub struct MicCapture {
    stream: Option<Stream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    device_sample_rate: u32,
    device_channels: u16,
}

impl MicCapture {
    /// Create a new mic capture using the default input device.
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("No input config: {e}"))?;

        log::info!(
            "Mic: {} ch, {} Hz, {:?}",
            config.channels(),
            config.sample_rate().0,
            config.sample_format()
        );

        Ok(Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            device_sample_rate: config.sample_rate().0,
            device_channels: config.channels(),
        })
    }

    /// Start recording. Clears any previous buffer.
    pub fn start(&mut self) -> Result<(), String> {
        // Clear buffer
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("No input config: {e}"))?;

        let sample_format = config.sample_format();
        let stream_config: StreamConfig = config.into();
        let buffer = Arc::clone(&self.buffer);
        let channels = self.device_channels;

        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        // Mix to mono
                        let mono: Vec<f32> = data
                            .chunks(channels as usize)
                            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                            .collect();
                        if let Ok(mut buf) = buffer.lock() {
                            buf.extend_from_slice(&mono);
                        }
                    },
                    |err| log::error!("Mic stream error: {err}"),
                    None,
                )
                .map_err(|e| format!("Failed to build input stream: {e}"))?,
            SampleFormat::I16 => {
                let buffer = Arc::clone(&self.buffer);
                device
                    .build_input_stream(
                        &stream_config,
                        move |data: &[i16], _| {
                            let mono: Vec<f32> = data
                                .chunks(channels as usize)
                                .map(|frame| {
                                    frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>()
                                        / channels as f32
                                })
                                .collect();
                            if let Ok(mut buf) = buffer.lock() {
                                buf.extend_from_slice(&mono);
                            }
                        },
                        |err| log::error!("Mic stream error: {err}"),
                        None,
                    )
                    .map_err(|e| format!("Failed to build input stream: {e}"))?
            }
            _ => return Err(format!("Unsupported sample format: {sample_format:?}")),
        };

        stream
            .play()
            .map_err(|e| format!("Failed to start mic stream: {e}"))?;

        self.stream = Some(stream);
        Ok(())
    }

    /// Stop recording and return the captured audio resampled to 16kHz mono.
    pub fn stop(&mut self) -> Vec<f32> {
        // Drop the stream to stop recording
        self.stream = None;

        let raw = {
            let mut buf = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *buf)
        };

        if raw.is_empty() {
            return raw;
        }

        // Resample to 16kHz if needed
        if self.device_sample_rate == WHISPER_SAMPLE_RATE {
            return raw;
        }

        resample(&raw, self.device_sample_rate, WHISPER_SAMPLE_RATE)
    }

    /// Check if currently recording.
    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }
}

/// Simple linear interpolation resampler.
fn resample(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;

        let sample = if idx + 1 < input.len() {
            input[idx] as f64 * (1.0 - frac) + input[idx + 1] as f64 * frac
        } else if idx < input.len() {
            input[idx] as f64
        } else {
            0.0
        };

        output.push(sample as f32);
    }

    output
}
