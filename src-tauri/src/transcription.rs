use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, Mutex};

#[cfg(not(any(target_os = "ios", target_os = "android")))]
use std::path::PathBuf;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const DEEPGRAM_WS_URL: &str = "wss://api.deepgram.com/v1/listen";
const ASSEMBLYAI_URL: &str = "https://api.assemblyai.com/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEvent {
    pub text: String,
    pub is_final: bool,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    channel: Option<DeepgramChannel>,
    is_final: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
struct DeepgramAlternative {
    transcript: String,
    confidence: f32,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct AssemblyAiUploadResponse {
    upload_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AssemblyAiTranscript {
    id: Option<String>,
    status: Option<String>,
    text: Option<String>,
    error: Option<String>,
}

#[derive(Default)]
pub struct TranscriptionState {
    pub deepgram_sender: Option<mpsc::Sender<Vec<i16>>>,
    pub is_streaming: bool,
}

impl TranscriptionState {
    /// Send audio samples directly (bypassing frontend to avoid JSON corruption)
    pub fn send_audio_direct(&self, samples: Vec<i16>) -> Result<(), String> {
        if let Some(sender) = &self.deepgram_sender {
            sender.try_send(samples).map_err(|e| format!("Failed to send: {}", e))
        } else {
            Ok(())
        }
    }
}

pub type TranscriptionManager = Arc<Mutex<TranscriptionState>>;

#[tauri::command]
pub async fn start_deepgram_stream(
    app: AppHandle,
    api_key: String,
    state: tauri::State<'_, TranscriptionManager>,
) -> Result<(), String> {

    // Atomically check and set streaming state to prevent race conditions
    {
        let mut state_guard = state.lock().await;
        if state_guard.is_streaming {
            tracing::warn!("Deepgram stream already active, skipping");
            return Err("Deepgram stream already active".to_string());
        }
        // Mark as streaming immediately to prevent duplicate connections
        state_guard.is_streaming = true;
    }

    let url = format!(
        "{}?model=nova-2&language=de&encoding=linear16&sample_rate=16000&channels=1&interim_results=true&punctuate=true&smart_format=true&endpointing=300",
        DEEPGRAM_WS_URL
    );

    let request = async_tungstenite::tungstenite::http::Request::builder()
        .uri(&url)
        .header("Authorization", format!("Token {}", api_key))
        .header("Sec-WebSocket-Key", async_tungstenite::tungstenite::handshake::client::generate_key())
        .header("Sec-WebSocket-Version", "13")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Host", "api.deepgram.com")
        .body(())
        .map_err(|e| format!("Failed to build request: {}", e))?;

    let (ws_stream, _) = match connect_async(request).await {
        Ok(stream) => stream,
        Err(e) => {
            // Reset streaming state on connection failure
            let mut state_guard = state.lock().await;
            state_guard.is_streaming = false;
            return Err(format!("Failed to connect to Deepgram: {}", e));
        }
    };

    let (mut write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(100);

    // Store the sender in state so audio forwarding can use it
    {
        let mut state_guard = state.lock().await;
        state_guard.deepgram_sender = Some(tx);
    }

    let app_clone = app.clone();
    let state_clone = state.inner().clone();

    // Spawn task to receive transcripts
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<DeepgramResponse>(&text) {
                        Ok(response) => {
                            if let Some(channel) = response.channel {
                                if let Some(alt) = channel.alternatives.first() {
                                    if !alt.transcript.is_empty() {
                                        let _ = app_clone.emit(
                                            "transcript",
                                            TranscriptEvent {
                                                text: alt.transcript.clone(),
                                                is_final: response.is_final.unwrap_or(false),
                                                confidence: alt.confidence,
                                                source: "deepgram".to_string(),
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse Deepgram response: {}", e);
                        }
                    }
                }
                Ok(Message::Close(frame)) => {
                    tracing::debug!("Deepgram connection closed: {:?}", frame);
                    break;
                }
                Ok(_) => {} // Ignore ping/pong/binary
                Err(e) => {
                    tracing::error!("Deepgram WebSocket error: {}", e);
                    break;
                }
            }
        }

        let mut state_guard = state_clone.lock().await;
        state_guard.is_streaming = false;
    });

    // Spawn task to send audio
    tokio::spawn(async move {
        while let Some(samples) = rx.recv().await {
            let bytes: Vec<u8> = samples
                .iter()
                .flat_map(|s| s.to_le_bytes())
                .collect();

            if let Err(e) = write.send(Message::Binary(bytes)).await {
                tracing::error!("Failed to send audio to Deepgram: {}", e);
                break;
            }
        }
        let _ = write.send(Message::Close(None)).await;
    });

    app.emit("deepgram-connected", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn send_audio_to_deepgram(
    samples: Vec<i16>,
    state: tauri::State<'_, TranscriptionManager>,
) -> Result<(), String> {
    let state_guard = state.lock().await;
    if let Some(sender) = &state_guard.deepgram_sender {
        sender.send(samples).await.map_err(|e| format!("Failed to send audio: {}", e))?;
        Ok(())
    } else {
        Err("Deepgram stream not active".to_string())
    }
}

#[tauri::command]
pub async fn stop_deepgram_stream(
    state: tauri::State<'_, TranscriptionManager>,
) -> Result<(), String> {
    let mut state_guard = state.lock().await;

    if !state_guard.is_streaming {
        return Ok(()); // Already stopped
    }

    state_guard.deepgram_sender = None;
    state_guard.is_streaming = false;
    Ok(())
}

#[tauri::command]
pub async fn is_deepgram_streaming(
    state: tauri::State<'_, TranscriptionManager>,
) -> Result<bool, String> {
    let state_guard = state.lock().await;
    Ok(state_guard.is_streaming)
}

#[tauri::command]
pub async fn transcribe_with_assemblyai(
    app: AppHandle,
    api_key: String,
    audio_data: Vec<i16>,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Convert to bytes
    let audio_bytes: Vec<u8> = audio_data
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();

    // Upload audio
    let upload_response = client
        .post(format!("{}/upload", ASSEMBLYAI_URL))
        .header("Authorization", &api_key)
        .header("Content-Type", "application/octet-stream")
        .body(audio_bytes)
        .send()
        .await
        .map_err(|e| format!("Upload failed: {}", e))?;

    let upload_result: serde_json::Value = upload_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse upload response: {}", e))?;

    let upload_url = upload_result["upload_url"]
        .as_str()
        .ok_or("No upload URL returned")?;

    // Create transcript
    let transcript_request = serde_json::json!({
        "audio_url": upload_url,
        "speech_model": "nano"
    });

    let create_response = client
        .post(format!("{}/transcript", ASSEMBLYAI_URL))
        .header("Authorization", &api_key)
        .header("Content-Type", "application/json")
        .json(&transcript_request)
        .send()
        .await
        .map_err(|e| format!("Create transcript failed: {}", e))?;

    let create_result: AssemblyAiTranscript = create_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse create response: {}", e))?;

    let transcript_id = create_result.id.ok_or("No transcript ID returned")?;

    // Poll for completion
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let poll_response = client
            .get(format!("{}/transcript/{}", ASSEMBLYAI_URL, transcript_id))
            .header("Authorization", &api_key)
            .send()
            .await
            .map_err(|e| format!("Poll failed: {}", e))?;

        let poll_result: AssemblyAiTranscript = poll_response
            .json()
            .await
            .map_err(|e| format!("Failed to parse poll response: {}", e))?;

        match poll_result.status.as_deref() {
            Some("completed") => {
                let text = poll_result.text.unwrap_or_default();
                let _ = app.emit(
                    "transcript",
                    TranscriptEvent {
                        text: text.clone(),
                        is_final: true,
                        confidence: 0.9,
                        source: "assemblyai".to_string(),
                    },
                );
                return Ok(text);
            }
            Some("error") => {
                return Err(poll_result.error.unwrap_or("Unknown error".to_string()));
            }
            _ => continue,
        }
    }
}

// ============================================================================
// Local Whisper Transcription (Desktop Only)
// ============================================================================

/// Get the path to the Whisper model file
#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn get_model_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;

    Ok(app_data_dir.join("ggml-base.en.bin"))
}

/// Download the Whisper model if not present
#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn ensure_model_exists(model_path: &PathBuf) -> Result<(), String> {
    if model_path.exists() {
        tracing::info!("Whisper model found at {:?}", model_path);
        return Ok(());
    }

    tracing::info!("Downloading Whisper model (base.en)...");

    let model_url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";

    let response = reqwest::get(model_url)
        .await
        .map_err(|e| format!("Failed to download model: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to download model: HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read model bytes: {}", e))?;

    std::fs::write(model_path, &bytes)
        .map_err(|e| format!("Failed to save model: {}", e))?;

    tracing::info!("Whisper model downloaded successfully");
    Ok(())
}

/// Convert i16 PCM samples to f32 (normalized to -1.0 to 1.0)
#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn convert_i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / i16::MAX as f32)
        .collect()
}

/// Transcribe audio using local Whisper model (Desktop only)
#[cfg(not(any(target_os = "ios", target_os = "android")))]
#[tauri::command]
pub async fn transcribe_local_whisper(
    app: AppHandle,
    audio_data: Vec<i16>,
) -> Result<String, String> {
    let model_path = get_model_path(&app)?;

    // Download model if needed
    ensure_model_exists(&model_path).await?;

    // Convert audio to f32
    let audio_f32 = convert_i16_to_f32(&audio_data);

    // Run inference in a blocking task to not block the async runtime
    let result = tokio::task::spawn_blocking(move || {
        // Create Whisper context
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| format!("Failed to create Whisper context: {}", e))?;

        // Create inference state
        let mut state = ctx
            .create_state()
            .map_err(|e| format!("Failed to create Whisper state: {}", e))?;

        // Set up parameters for transcription
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Configure for English, single segment output
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(false);
        params.set_no_context(true);

        // Run inference
        state
            .full(params, &audio_f32)
            .map_err(|e| format!("Whisper inference failed: {}", e))?;

        // Collect all segments
        let num_segments = state.full_n_segments().map_err(|e| format!("Failed to get segments: {}", e))?;
        let mut transcript = String::new();

        for i in 0..num_segments {
            if let Ok(segment_text) = state.full_get_segment_text(i) {
                transcript.push_str(&segment_text);
                transcript.push(' ');
            }
        }

        Ok::<String, String>(transcript.trim().to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    // Emit transcript event
    let _ = app.emit(
        "transcript",
        TranscriptEvent {
            text: result.clone(),
            is_final: true,
            confidence: 0.85, // Local model doesn't provide confidence
            source: "whisper-local".to_string(),
        },
    );

    tracing::info!("Local Whisper transcription complete: {} chars", result.len());
    Ok(result)
}
