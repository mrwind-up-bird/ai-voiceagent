//! Desktop audio capture using CPAL.
//!
//! Supports macOS, Windows, and Linux with native audio APIs.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::{AudioCapture, AudioCaptureError, AudioChunk, TARGET_SAMPLE_RATE};

/// Desktop audio capture implementation using CPAL
pub struct DesktopAudioCapture {
    is_recording: Arc<AtomicBool>,
    current_device: Arc<Mutex<Option<String>>>,
}

impl DesktopAudioCapture {
    /// Create a new desktop audio capture instance
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            current_device: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for DesktopAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple linear resampling from source rate to target rate
fn resample(samples: &[i16], source_rate: u32, target_rate: u32) -> Vec<i16> {
    if source_rate == target_rate {
        return samples.to_vec();
    }

    let ratio = source_rate as f64 / target_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = (i as f64 * ratio) as usize;
        if src_idx < samples.len() {
            output.push(samples[src_idx]);
        }
    }

    output
}

impl AudioCapture for DesktopAudioCapture {
    fn start(&self, callback: Box<dyn Fn(AudioChunk) + Send + Sync>) -> Result<(), AudioCaptureError> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err(AudioCaptureError::Stream("Already recording".to_string()));
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(AudioCaptureError::NoDevice)?;

        // Store device name
        if let Ok(name) = device.name() {
            if let Ok(mut current) = self.current_device.lock() {
                *current = Some(name);
            }
        }

        // Get supported config
        let mut supported_configs = device
            .supported_input_configs()
            .map_err(|e| AudioCaptureError::Configuration(e.to_string()))?;

        // Find a config that supports our target sample rate, or fall back to default
        let config: cpal::StreamConfig = supported_configs
            .find(|c| {
                c.min_sample_rate().0 <= TARGET_SAMPLE_RATE
                    && c.max_sample_rate().0 >= TARGET_SAMPLE_RATE
            })
            .map(|c| c.with_sample_rate(cpal::SampleRate(TARGET_SAMPLE_RATE)).into())
            .unwrap_or_else(|| {
                let default = device.default_input_config().unwrap();
                tracing::warn!(
                    "16kHz not supported, using device default: {}Hz",
                    default.sample_rate().0
                );
                default.into()
            });

        let actual_sample_rate = config.sample_rate.0;
        let needs_resampling = actual_sample_rate != TARGET_SAMPLE_RATE;
        let channels = config.channels as usize;

        let is_recording = self.is_recording.clone();
        is_recording.store(true, Ordering::SeqCst);

        let buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();

        // Calculate chunk size based on actual sample rate (~100ms of audio)
        let chunk_size = (actual_sample_rate as usize) / 10;

        let callback = Arc::new(callback);
        let callback_clone = callback.clone();

        // Clone is_recording for inner closure
        let is_recording_inner = is_recording.clone();

        // Spawn audio capture thread
        std::thread::spawn(move || {
            let stream = device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if !is_recording_inner.load(Ordering::SeqCst) {
                            return;
                        }

                        // Convert stereo to mono if needed
                        let mono_samples: Vec<f32> = if channels > 1 {
                            data.chunks(channels).map(|chunk| chunk[0]).collect()
                        } else {
                            data.to_vec()
                        };

                        // Convert f32 to i16 PCM
                        let samples: Vec<i16> = mono_samples
                            .iter()
                            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                            .collect();

                        if let Ok(mut buf) = buffer_clone.lock() {
                            buf.extend(samples);

                            // Send audio chunk every ~100ms
                            if buf.len() >= chunk_size {
                                let chunk: Vec<i16> = buf.drain(..).collect();

                                // Resample to 16kHz if needed
                                let resampled = if needs_resampling {
                                    resample(&chunk, actual_sample_rate, TARGET_SAMPLE_RATE)
                                } else {
                                    chunk
                                };

                                callback_clone(AudioChunk {
                                    samples: resampled,
                                    sample_rate: TARGET_SAMPLE_RATE,
                                    channels: 1,
                                });
                            }
                        }
                    },
                    |err| {
                        tracing::error!("Audio stream error: {}", err);
                    },
                    None,
                )
                .expect("Failed to build input stream");

            stream.play().expect("Failed to start audio stream");

            // Keep stream alive while recording
            while is_recording.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // Stream is dropped here, stopping the recording
        });

        Ok(())
    }

    fn stop(&self) -> Result<(), AudioCaptureError> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Ok(()); // Already stopped
        }

        self.is_recording.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    fn list_devices(&self) -> Result<Vec<String>, AudioCaptureError> {
        let host = cpal::default_host();
        let devices: Vec<String> = host
            .input_devices()
            .map_err(|e| AudioCaptureError::Configuration(e.to_string()))?
            .filter_map(|d| d.name().ok())
            .collect();
        Ok(devices)
    }

    fn current_device(&self) -> Option<String> {
        self.current_device.lock().ok()?.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_same_rate() {
        let samples: Vec<i16> = vec![100, 200, 300, 400, 500];
        let result = resample(&samples, 16000, 16000);
        assert_eq!(result, samples);
    }

    #[test]
    fn test_resample_downsample() {
        let samples: Vec<i16> = vec![100, 200, 300, 400, 500, 600, 700, 800];
        let result = resample(&samples, 48000, 16000);
        // 48kHz -> 16kHz = 3:1 ratio, so 8 samples -> ~2-3 samples
        assert!(result.len() < samples.len());
    }
}
