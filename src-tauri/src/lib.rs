pub mod agents;
pub mod platform;
pub mod secrets;
pub mod transcription;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub mod audio;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub mod tts;

use std::sync::Arc;
use tauri::Manager;
use transcription::TranscriptionManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init());

    // Global shortcut plugin is desktop-only (not available on mobile)
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());

    builder
        .setup(|app| {
            // Initialize transcription state
            let transcription_state: TranscriptionManager =
                Arc::new(tokio::sync::Mutex::new(transcription::TranscriptionState::default()));
            app.manage(transcription_state);

            tracing::info!("API keys stored in OS secure storage (Keychain/Credential Manager/Keystore)");

            // Register global shortcut (Cmd+Shift+V for Voice) - Desktop only
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

                let shortcut = Shortcut::new(
                    Some(Modifiers::SUPER | Modifiers::SHIFT),
                    Code::KeyV,
                );

                let app_handle = app.handle().clone();

                if let Err(e) = app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
                    // Only handle key press, not key release
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if let Some(window) = app_handle.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.center();
                        }
                    }
                }) {
                    tracing::warn!("Failed to set shortcut handler: {}", e);
                }

                if let Err(e) = app.global_shortcut().register(shortcut) {
                    tracing::warn!("Failed to register global shortcut Cmd+Shift+V: {}", e);
                    // App will still work, just without the global shortcut
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Secrets commands
            secrets::set_api_key,
            secrets::get_api_key,
            secrets::delete_api_key,
            secrets::has_api_keys,
            secrets::list_configured_keys,
            // Audio commands (desktop only)
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            audio::start_recording,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            audio::stop_recording,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            audio::is_recording,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            audio::list_audio_devices,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            audio::save_recording,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            audio::has_recording,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            audio::get_recording_duration,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            audio::clear_recording_buffer,
            // Transcription commands
            transcription::start_deepgram_stream,
            transcription::stop_deepgram_stream,
            transcription::send_audio_to_deepgram,
            transcription::is_deepgram_streaming,
            transcription::transcribe_with_assemblyai,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            transcription::transcribe_local_whisper,
            // Action Items agent
            agents::action_items::extract_action_items,
            agents::action_items::extract_action_items_streaming,
            // Tone Shifter agent
            agents::tone_shifter::shift_tone,
            agents::tone_shifter::shift_tone_streaming,
            agents::tone_shifter::get_available_tones,
            agents::tone_shifter::get_tone_presets,
            // Music Matcher agent
            agents::music_matcher::match_music,
            agents::music_matcher::analyze_mood_from_transcript,
            agents::music_matcher::get_available_moods,
            agents::music_matcher::get_available_genres,
            // Translator agent
            agents::translator::translate_text,
            agents::translator::translate_text_streaming,
            agents::translator::get_available_languages,
            // Dev-Log agent
            agents::dev_log::generate_dev_log,
            agents::dev_log::generate_dev_log_streaming,
            // Brain Dump agent
            agents::brain_dump::process_brain_dump,
            agents::brain_dump::process_brain_dump_streaming,
            // Mental Mirror agent (Letter to Myself)
            agents::mental_mirror::generate_mental_mirror,
            agents::mental_mirror::generate_mental_mirror_streaming,
            agents::mental_mirror::schedule_mental_mirror_email,
            agents::mental_mirror::export_letter_to_file,
            // Text-to-Speech (desktop only)
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            tts::speak_text,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            tts::stop_speech,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            tts::is_speaking,
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            tts::get_available_voices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
