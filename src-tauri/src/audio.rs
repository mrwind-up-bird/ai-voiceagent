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

/// RAII guard that ensures the recording flag is cleared even if the
/// capture thread panics (C3, H3). Parameterised over the flag so tests
/// can exercise it without touching global state.
struct RecordingGuard<'a> {
    flag: &'a AtomicBool,
}

impl<'a> RecordingGuard<'a> {
    fn try_acquire(flag: &'a AtomicBool) -> Result<Self, String> {
        if flag.swap(true, Ordering::SeqCst) {
            return Err("Already recording".to_string());
        }
        Ok(Self { flag })
    }
}

impl<'a> Drop for RecordingGuard<'a> {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

fn panic_to_msg(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return (*s).to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "audio capture thread panicked (unknown payload)".to_string()
}

/// Supervise the audio capture loop. Returns Err if the CPAL error
/// callback flipped `stream_healthy` to false (H3 — previously the
/// callback only logged, leaving UI stuck in "recording"). Extracted
/// for unit-testing without a real cpal stream.
fn wait_for_capture_end(
    is_recording: &AtomicBool,
    stream_healthy: &AtomicBool,
    tick: std::time::Duration,
) -> Result<(), String> {
    while is_recording.load(Ordering::SeqCst) && stream_healthy.load(Ordering::SeqCst) {
        std::thread::sleep(tick);
    }
    if !stream_healthy.load(Ordering::SeqCst) {
        return Err(
            "audio stream errored (device unplugged, exclusive-mode contention, or driver hiccup)"
                .to_string(),
        );
    }
    Ok(())
}

fn calculate_energy(samples: &[f32]) -> f32 {
    // Audio PCM in f32 is contractually in [-1, 1]. Filter NaN/Inf and
    // clamp out-of-range samples so a misbehaving driver delivering
    // hostile floats cannot poison the VAD threshold (L1).
    let (sum, count) = samples.iter().fold((0.0_f32, 0_usize), |(acc, n), s| {
        if s.is_finite() {
            let clamped = s.clamp(-1.0, 1.0);
            (acc + clamped * clamped, n + 1)
        } else {
            (acc, n)
        }
    });
    if count == 0 {
        return 0.0;
    }
    (sum / count as f32).sqrt()
}

#[tauri::command]
pub fn start_recording(app: AppHandle) -> Result<(), String> {
    // Acquire the recording flag through a guard that resets it even
    // on panic-unwind from the spawned thread (C3).
    let guard = RecordingGuard::try_acquire(&IS_RECORDING)?;

    // Clear the recording buffer for a new recording
    if let Ok(mut buffer) = RECORDING_BUFFER.lock() {
        buffer.clear();
    }

    // Spawn a dedicated thread for audio capture (cpal::Stream is not Send)
    std::thread::spawn(move || {
        // The guard is moved into the thread; Drop runs on normal exit,
        // early return, AND panic-unwind, always clearing IS_RECORDING.
        let _guard = guard;
        let app_for_err = app.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_audio_capture(app)
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                tracing::error!("Audio capture error: {}", e);
                let _ = app_for_err.emit("recording-error", e);
            }
            Err(panic_payload) => {
                let msg = panic_to_msg(&panic_payload);
                tracing::error!("Audio capture thread panicked: {}", msg);
                let _ = app_for_err.emit(
                    "recording-error",
                    format!("audio capture panicked: {}", msg),
                );
            }
        }
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
        .map(Ok)
        .unwrap_or_else(|| {
            // Fall back to default config if 16kHz isn't supported (C3:
            // propagate the error instead of panicking on devices with
            // no retrievable default config — e.g. a USB mic unplugged
            // between start() returning Ok and the spawned thread
            // reaching this line).
            let default = device
                .default_input_config()
                .map_err(|e| format!("No default input config for audio device: {}", e))?;
            tracing::warn!(
                "16kHz not supported, using device default: {}Hz",
                default.sample_rate().0
            );
            Ok::<cpal::StreamConfig, String>(default.into())
        })?;

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

    // H3: shared health flag flipped to false from the CPAL error
    // callback so the supervisor loop can break and emit recording-error.
    let stream_healthy = Arc::new(AtomicBool::new(true));
    let stream_healthy_err = stream_healthy.clone();

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
            move |err| {
                tracing::error!("Audio stream error: {}", err);
                // H3: signal the supervisor loop so it exits + emits
                // recording-error instead of leaving UI stuck.
                stream_healthy_err.store(false, Ordering::SeqCst);
            },
            None,
        )
        .map_err(|e| format!("Failed to build stream: {}", e))?;

    stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;

    let _ = app.emit("recording-started", ());

    // Supervised loop — returns Err if stream errored mid-session.
    let supervise_result = wait_for_capture_end(
        &IS_RECORDING,
        &stream_healthy,
        std::time::Duration::from_millis(100),
    );

    // Stream is dropped here, stopping the recording
    let _ = app.emit("recording-stopped", ());

    supervise_result
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

    // Validate filepath: must be within home dir or app data, must end in .wav
    let path = std::path::Path::new(&filepath);

    // Enforce .wav extension
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("wav") => {},
        _ => return Err("File must have .wav extension".to_string()),
    }

    // Resolve to canonical path (resolves ../ and symlinks)
    let parent = path.parent()
        .ok_or("Invalid file path")?;
    let canonical_parent = parent.canonicalize()
        .map_err(|e| format!("Invalid directory path: {}", e))?;

    // Check path is within allowed directories
    let home_dir = dirs::home_dir()
        .ok_or("Could not determine home directory")?;
    let data_dir = dirs::data_dir()
        .ok_or("Could not determine app data directory")?;

    if !canonical_parent.starts_with(&home_dir) && !canonical_parent.starts_with(&data_dir) {
        return Err("File path must be within home or app data directory".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn calculate_energy_empty_returns_zero() {
        assert_eq!(calculate_energy(&[]), 0.0);
    }

    // C3 regression tests — RecordingGuard must clear the flag on any
    // exit path including panic-unwind, and must reject double-acquire.

    #[test]
    fn guard_resets_flag_on_drop() {
        let flag = AtomicBool::new(false);
        {
            let _g = RecordingGuard::try_acquire(&flag).expect("acquire");
            assert!(flag.load(Ordering::SeqCst));
        }
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn guard_resets_flag_on_panic_unwind() {
        let flag = AtomicBool::new(false);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = RecordingGuard::try_acquire(&flag).expect("acquire");
            assert!(flag.load(Ordering::SeqCst));
            panic!("simulated audio thread panic");
        }));
        assert!(result.is_err(), "panic must propagate out of closure");
        assert!(
            !flag.load(Ordering::SeqCst),
            "RecordingGuard must reset flag during unwind"
        );
    }

    #[test]
    fn guard_rejects_double_acquire() {
        let flag = AtomicBool::new(false);
        let _g = RecordingGuard::try_acquire(&flag).expect("first acquire");
        assert!(
            RecordingGuard::try_acquire(&flag).is_err(),
            "second acquire while held must fail"
        );
    }

    #[test]
    fn guard_releases_then_reacquires() {
        let flag = AtomicBool::new(false);
        {
            let _g = RecordingGuard::try_acquire(&flag).expect("first");
        }
        let _g2 = RecordingGuard::try_acquire(&flag).expect("second after drop");
    }

    // H3 — wait_for_capture_end supervisor tests

    #[test]
    fn supervisor_returns_ok_on_normal_stop() {
        let is_rec = Arc::new(AtomicBool::new(true));
        let healthy = Arc::new(AtomicBool::new(true));
        let is_rec_c = is_rec.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            is_rec_c.store(false, Ordering::SeqCst);
        });
        let res = wait_for_capture_end(&is_rec, &healthy, std::time::Duration::from_millis(5));
        assert!(res.is_ok(), "normal stop should return Ok, got: {:?}", res);
    }

    #[test]
    fn supervisor_returns_err_on_stream_unhealthy() {
        let is_rec = Arc::new(AtomicBool::new(true));
        let healthy = Arc::new(AtomicBool::new(true));
        let healthy_c = healthy.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            healthy_c.store(false, Ordering::SeqCst);
        });
        let res = wait_for_capture_end(&is_rec, &healthy, std::time::Duration::from_millis(5));
        assert!(res.is_err(), "unhealthy stream must produce Err");
        let msg = res.unwrap_err();
        assert!(
            msg.contains("audio stream errored"),
            "error message should mention stream error: {}",
            msg
        );
    }

    #[test]
    fn supervisor_returns_immediately_if_already_stopped() {
        let is_rec = AtomicBool::new(false);
        let healthy = AtomicBool::new(true);
        let start = std::time::Instant::now();
        let res = wait_for_capture_end(&is_rec, &healthy, std::time::Duration::from_millis(100));
        assert!(res.is_ok());
        assert!(start.elapsed() < std::time::Duration::from_millis(50));
    }

    #[test]
    fn panic_to_msg_extracts_str_payload() {
        let payload: Box<dyn std::any::Any + Send> = Box::new("boom static str");
        assert_eq!(panic_to_msg(&payload), "boom static str");
    }

    #[test]
    fn panic_to_msg_extracts_string_payload() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(String::from("boom string"));
        assert_eq!(panic_to_msg(&payload), "boom string");
    }

    #[test]
    fn panic_to_msg_falls_back_for_unknown_payload() {
        #[derive(Debug)]
        struct Weird;
        let payload: Box<dyn std::any::Any + Send> = Box::new(Weird);
        assert!(panic_to_msg(&payload).contains("unknown payload"));
    }

    #[test]
    fn resample_empty_returns_empty() {
        assert!(resample(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn resample_identity_when_rates_equal() {
        let samples = vec![1_i16, 2, 3, -4, 5];
        assert_eq!(resample(&samples, 16_000, 16_000), samples);
    }

    proptest! {
        #[test]
        fn resample_never_panics_for_any_input(
            samples in proptest::collection::vec(any::<i16>(), 0..2_000),
            source_rate in 0u32..200_000,
            target_rate in 0u32..200_000,
        ) {
            let _ = resample(&samples, source_rate, target_rate);
        }

        #[test]
        fn resample_identity_proptest(
            samples in proptest::collection::vec(any::<i16>(), 0..1_000),
            rate in 1u32..200_000,
        ) {
            prop_assert_eq!(resample(&samples, rate, rate), samples);
        }

        #[test]
        fn calculate_energy_finite_for_bounded_input(
            samples in proptest::collection::vec(-1.0_f32..=1.0_f32, 0..2_000),
        ) {
            let e = calculate_energy(&samples);
            prop_assert!(e.is_finite(), "energy not finite for bounded input: {}", e);
            prop_assert!(e >= 0.0, "energy negative: {}", e);
        }

        #[test]
        fn calculate_energy_never_panics_for_any_finite_input(
            samples in proptest::collection::vec(prop_oneof![
                -1e10_f32..=1e10_f32,
                Just(0.0_f32),
            ], 0..1_000),
        ) {
            let _ = calculate_energy(&samples);
        }

        #[test]
        fn calculate_energy_handles_nan_and_inf_without_panic(
            samples in proptest::collection::vec(prop_oneof![
                Just(f32::NAN),
                Just(f32::INFINITY),
                Just(f32::NEG_INFINITY),
                any::<f32>(),
            ], 0..200),
        ) {
            // L1: NaN/Inf may produce NaN output today (documented bug).
            // After fix, output must be finite even on hostile input.
            let e = calculate_energy(&samples);
            prop_assert!(e.is_finite(), "energy not finite on NaN/Inf input: {}", e);
            prop_assert!(e >= 0.0, "energy negative on NaN/Inf input: {}", e);
        }
    }
}
