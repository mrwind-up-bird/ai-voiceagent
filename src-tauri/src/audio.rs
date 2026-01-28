use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use once_cell::sync::Lazy;

use crate::transcription::TranscriptionManager;

const TARGET_SAMPLE_RATE: u32 = 16000;
const VAD_THRESHOLD: f32 = 0.02;

/// Global buffer to store all recorded audio samples for saving
static RECORDING_BUFFER: Lazy<Mutex<Vec<i16>>> = Lazy::new(|| Mutex::new(Vec::new()));

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

#[derive(Clone, serde::Serialize)]
pub struct AudioChunk {
    pub data: Vec<i16>,
    pub sample_rate: u32,
}

#[derive(Clone, serde::Serialize)]
pub struct VadEvent {
    pub is_speech: bool,
    pub energy: f32,
}

// Global flag for recording state - this is safe because it's just an atomic bool
static IS_RECORDING: AtomicBool = AtomicBool::new(false);

fn calculate_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

#[tauri::command]
pub fn start_recording(app: AppHandle) -> Result<(), String> {
    if IS_RECORDING.load(Ordering::SeqCst) {
        return Err("Already recording".to_string());
    }

    // Clear the recording buffer for a new recording
    if let Ok(mut buffer) = RECORDING_BUFFER.lock() {
        buffer.clear();
    }

    IS_RECORDING.store(true, Ordering::SeqCst);

    // Spawn a dedicated thread for audio capture (cpal::Stream is not Send)
    std::thread::spawn(move || {
        let result = run_audio_capture(app.clone());
        if let Err(e) = result {
            tracing::error!("Audio capture error: {}", e);
            let _ = app.emit("recording-error", e);
        }
        IS_RECORDING.store(false, Ordering::SeqCst);
    });

    Ok(())
}

fn run_audio_capture(app: AppHandle) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;

    // Try to get a config with our target sample rate (16kHz for Deepgram)
    let mut supported_configs = device
        .supported_input_configs()
        .map_err(|e| format!("Failed to get supported configs: {}", e))?;

    // Find a config that supports our target sample rate, or fall back to default
    let config: cpal::StreamConfig = supported_configs
        .find(|c| {
            c.min_sample_rate().0 <= TARGET_SAMPLE_RATE && c.max_sample_rate().0 >= TARGET_SAMPLE_RATE
        })
        .map(|c| c.with_sample_rate(cpal::SampleRate(TARGET_SAMPLE_RATE)).into())
        .unwrap_or_else(|| {
            // Fall back to default config if 16kHz isn't supported
            let default = device.default_input_config().unwrap();
            tracing::warn!(
                "16kHz not supported, using device default: {}Hz",
                default.sample_rate().0
            );
            default.into()
        });

    let actual_sample_rate = config.sample_rate.0;
    let needs_resampling = actual_sample_rate != TARGET_SAMPLE_RATE;

    let app_clone = app.clone();
    let buffer = Arc::new(std::sync::Mutex::new(Vec::<i16>::new()));
    let buffer_clone = buffer.clone();
    let channels = config.channels as usize;

    // Get transcription state for direct audio forwarding
    let transcription_state: TranscriptionManager = app.state::<TranscriptionManager>().inner().clone();

    // Calculate chunk size based on actual sample rate (~100ms of audio)
    let chunk_size = (actual_sample_rate as usize) / 10; // 100ms worth of samples

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !IS_RECORDING.load(Ordering::SeqCst) {
                    return;
                }

                // Convert stereo to mono if needed
                let mono_samples: Vec<f32> = if channels > 1 {
                    data.chunks(channels)
                        .map(|chunk| chunk[0]) // Take first channel
                        .collect()
                } else {
                    data.to_vec()
                };

                let energy = calculate_energy(&mono_samples);
                let is_speech = energy > VAD_THRESHOLD;

                // Emit VAD event
                let _ = app_clone.emit("vad-event", VadEvent { is_speech, energy });

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

                        // Resample to 16kHz if needed (for Deepgram compatibility)
                        let resampled = if needs_resampling {
                            resample(&chunk, actual_sample_rate, TARGET_SAMPLE_RATE)
                        } else {
                            chunk
                        };

                        // Store in global recording buffer for later saving
                        if let Ok(mut rec_buffer) = RECORDING_BUFFER.lock() {
                            rec_buffer.extend(resampled.iter());
                        }

                        // Send directly to Deepgram (bypassing frontend JSON serialization)
                        if let Ok(state) = transcription_state.try_lock() {
                            if state.is_streaming {
                                let _ = state.send_audio_direct(resampled.clone());
                            }
                        }

                        // Also emit for frontend visualization (but not for transcription)
                        let _ = app_clone.emit(
                            "audio-chunk",
                            AudioChunk {
                                data: resampled,
                                sample_rate: TARGET_SAMPLE_RATE,
                            },
                        );
                    }
                }
            },
            |err| {
                tracing::error!("Audio stream error: {}", err);
            },
            None,
        )
        .map_err(|e| format!("Failed to build stream: {}", e))?;

    stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;

    let _ = app.emit("recording-started", ());

    // Keep stream alive while recording
    while IS_RECORDING.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Stream is dropped here, stopping the recording
    let _ = app.emit("recording-stopped", ());

    Ok(())
}

#[tauri::command]
pub fn stop_recording(_app: AppHandle) -> Result<(), String> {
    if !IS_RECORDING.load(Ordering::SeqCst) {
        return Err("Not recording".to_string());
    }

    tracing::info!("stop stream");
    
    IS_RECORDING.store(false, Ordering::SeqCst);

    // The recording thread will emit recording-stopped when it exits
    Ok(())
}

#[tauri::command]
pub fn is_recording() -> bool {
    IS_RECORDING.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let devices: Vec<String> = host
        .input_devices()
        .map_err(|e| e.to_string())?
        .filter_map(|d| d.name().ok())
        .collect();
    Ok(devices)
}

/// Save the recorded audio buffer to a WAV file
#[tauri::command]
pub fn save_recording(app: AppHandle, filepath: String) -> Result<(), String> {
    let samples = {
        let buffer = RECORDING_BUFFER
            .lock()
            .map_err(|_| "Failed to lock recording buffer")?;
        buffer.clone()
    };

    if samples.is_empty() {
        return Err("No audio recorded".to_string());
    }

    // Create WAV spec for 16kHz mono 16-bit PCM
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(&filepath, spec)
        .map_err(|e| format!("Failed to create WAV file: {}", e))?;

    for sample in &samples {
        writer
            .write_sample(*sample)
            .map_err(|e| format!("Failed to write sample: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

    let duration_secs = samples.len() as f32 / TARGET_SAMPLE_RATE as f32;

    let _ = app.emit("recording-saved", serde_json::json!({
        "filepath": filepath,
        "duration_secs": duration_secs,
        "sample_count": samples.len()
    }));

    tracing::info!("Saved recording to {} ({:.1}s)", filepath, duration_secs);

    Ok(())
}

/// Check if there's recorded audio available to save
#[tauri::command]
pub fn has_recording() -> Result<bool, String> {
    let buffer = RECORDING_BUFFER
        .lock()
        .map_err(|_| "Failed to lock recording buffer")?;
    Ok(!buffer.is_empty())
}

/// Get the duration of the current recording buffer in seconds
#[tauri::command]
pub fn get_recording_duration() -> Result<f32, String> {
    let buffer = RECORDING_BUFFER
        .lock()
        .map_err(|_| "Failed to lock recording buffer")?;
    Ok(buffer.len() as f32 / TARGET_SAMPLE_RATE as f32)
}

/// Clear the recording buffer
#[tauri::command]
pub fn clear_recording_buffer() -> Result<(), String> {
    let mut buffer = RECORDING_BUFFER
        .lock()
        .map_err(|_| "Failed to lock recording buffer")?;
    buffer.clear();
    Ok(())
}
