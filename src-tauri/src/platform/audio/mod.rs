//! Cross-platform audio capture abstraction.
//!
//! Platform implementations:
//! - Desktop (macOS/Windows/Linux): CPAL for native audio capture
//! - Mobile (iOS/Android): Placeholder for native bridge implementation
//!
//! On mobile, audio capture is handled differently:
//! - Option A: Web Audio API in WebView (simpler, implemented in frontend)
//! - Option B: Native Swift/Kotlin bridges (better performance, future work)
//!
//! This module provides the Rust-side abstraction. For mobile, the frontend
//! handles audio capture and sends samples to Rust via Tauri commands.

use std::fmt;

/// Error type for audio capture operations
#[derive(Debug)]
pub enum AudioCaptureError {
    /// No audio input device available
    NoDevice,
    /// Device configuration error
    Configuration(String),
    /// Stream error during capture
    Stream(String),
    /// Permission denied
    PermissionDenied,
    /// Platform not supported for native capture
    NotSupported,
}

impl fmt::Display for AudioCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDevice => write!(f, "No audio input device available"),
            Self::Configuration(msg) => write!(f, "Audio configuration error: {}", msg),
            Self::Stream(msg) => write!(f, "Audio stream error: {}", msg),
            Self::PermissionDenied => write!(f, "Microphone permission denied"),
            Self::NotSupported => write!(f, "Native audio capture not supported on this platform"),
        }
    }
}

impl std::error::Error for AudioCaptureError {}

impl From<AudioCaptureError> for String {
    fn from(err: AudioCaptureError) -> Self {
        err.to_string()
    }
}

/// Audio sample format for captured audio
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// PCM samples (16-bit signed)
    pub samples: Vec<i16>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
}

/// Trait for platform-specific audio capture implementations
pub trait AudioCapture: Send + Sync {
    /// Start audio capture with a callback for received samples
    fn start(&self, callback: Box<dyn Fn(AudioChunk) + Send + Sync>) -> Result<(), AudioCaptureError>;

    /// Stop audio capture
    fn stop(&self) -> Result<(), AudioCaptureError>;

    /// Check if currently recording
    fn is_recording(&self) -> bool;

    /// List available audio input devices
    fn list_devices(&self) -> Result<Vec<String>, AudioCaptureError>;

    /// Get the current device name
    fn current_device(&self) -> Option<String>;
}

// Desktop implementation using CPAL
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod desktop;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub use desktop::DesktopAudioCapture;

// Mobile placeholder - actual capture happens in frontend via Web Audio API
#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile;

#[cfg(any(target_os = "ios", target_os = "android"))]
pub use mobile::MobileAudioCapture;

/// Check if native audio capture is available on this platform
pub fn is_native_capture_available() -> bool {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        true
    }
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        false
    }
}

/// Target sample rate for transcription (Deepgram/Whisper)
pub const TARGET_SAMPLE_RATE: u32 = 16000;

/// Voice Activity Detection threshold
pub const VAD_THRESHOLD: f32 = 0.02;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_capture_availability() {
        // On desktop, native capture should be available
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        assert!(is_native_capture_available());

        // On mobile, it should not be available (use Web Audio instead)
        #[cfg(any(target_os = "ios", target_os = "android"))]
        assert!(!is_native_capture_available());
    }
}
