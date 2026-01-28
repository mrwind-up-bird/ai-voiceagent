use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

// Flag to track if speech is currently active
static IS_SPEAKING: AtomicBool = AtomicBool::new(false);

/// Speak text using native TTS
/// On macOS, uses the `say` command which interfaces with AVSpeechSynthesizer
/// On Windows, uses PowerShell with SAPI
#[tauri::command]
pub async fn speak_text(
    app: AppHandle,
    text: String,
    rate: Option<f32>,
    voice: Option<String>,
) -> Result<(), String> {
    if text.is_empty() {
        return Err("No text to speak".to_string());
    }

    // Stop any current speech first
    let _ = stop_speech_internal();

    IS_SPEAKING.store(true, Ordering::SeqCst);
    let _ = app.emit("tts-started", ());

    let app_clone = app.clone();

    // Spawn blocking task for TTS
    tokio::task::spawn_blocking(move || {
        let result = speak_native(&text, rate, voice.as_deref());

        IS_SPEAKING.store(false, Ordering::SeqCst);
        let _ = app_clone.emit("tts-finished", ());

        result
    })
    .await
    .map_err(|e| format!("TTS task failed: {}", e))?
}

/// Stop any currently playing speech
#[tauri::command]
pub fn stop_speech() -> Result<(), String> {
    stop_speech_internal()
}

fn stop_speech_internal() -> Result<(), String> {
    if !IS_SPEAKING.load(Ordering::SeqCst) {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        // Kill any running `say` processes
        let _ = Command::new("pkill").arg("-9").arg("say").output();
    }

    #[cfg(target_os = "windows")]
    {
        // Stop SAPI speech
        let _ = Command::new("powershell")
            .args([
                "-Command",
                "Add-Type -AssemblyName System.Speech; $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer; $synth.SpeakAsyncCancelAll()",
            ])
            .output();
    }

    IS_SPEAKING.store(false, Ordering::SeqCst);
    Ok(())
}

/// Check if speech is currently active
#[tauri::command]
pub fn is_speaking() -> bool {
    IS_SPEAKING.load(Ordering::SeqCst)
}

/// Get available voices on the system
#[tauri::command]
pub fn get_available_voices() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("say")
            .arg("-v")
            .arg("?")
            .output()
            .map_err(|e| format!("Failed to list voices: {}", e))?;

        let voices: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                // Format: "Voice Name    language_code  # description"
                line.split_whitespace().next().map(|s| s.to_string())
            })
            .collect();

        Ok(voices)
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args([
                "-Command",
                "Add-Type -AssemblyName System.Speech; $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer; $synth.GetInstalledVoices() | ForEach-Object { $_.VoiceInfo.Name }",
            ])
            .output()
            .map_err(|e| format!("Failed to list voices: {}", e))?;

        let voices: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(voices)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("TTS not supported on this platform".to_string())
    }
}

#[cfg(target_os = "macos")]
fn speak_native(text: &str, rate: Option<f32>, voice: Option<&str>) -> Result<(), String> {
    let mut cmd = Command::new("say");

    // Set speech rate (words per minute, default ~175-200)
    if let Some(r) = rate {
        // Convert 0.5-2.0 scale to words per minute (100-350)
        let wpm = (r * 175.0).clamp(100.0, 350.0) as u32;
        cmd.arg("-r").arg(wpm.to_string());
    }

    // Set voice
    if let Some(v) = voice {
        cmd.arg("-v").arg(v);
    }

    cmd.arg(text);

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run say command: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("Speech synthesis failed".to_string())
    }
}

#[cfg(target_os = "windows")]
fn speak_native(text: &str, rate: Option<f32>, voice: Option<&str>) -> Result<(), String> {
    // Escape text for PowerShell
    let escaped_text = text.replace("'", "''").replace("`", "``");

    let mut script = String::from("Add-Type -AssemblyName System.Speech; $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer;");

    // Set voice if specified
    if let Some(v) = voice {
        script.push_str(&format!(" $synth.SelectVoice('{}');", v.replace("'", "''")));
    }

    // Set rate (-10 to 10, default 0)
    if let Some(r) = rate {
        let sapi_rate = ((r - 1.0) * 10.0).clamp(-10.0, 10.0) as i32;
        script.push_str(&format!(" $synth.Rate = {};", sapi_rate));
    }

    script.push_str(&format!(" $synth.Speak('{}')", escaped_text));

    let status = Command::new("powershell")
        .args(["-Command", &script])
        .status()
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("Speech synthesis failed".to_string())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn speak_native(_text: &str, _rate: Option<f32>, _voice: Option<&str>) -> Result<(), String> {
    Err("TTS not supported on this platform".to_string())
}
