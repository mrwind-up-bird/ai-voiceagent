//! Platform abstraction layer for cross-platform functionality.
//!
//! This module provides unified interfaces for platform-specific features:
//! - Secrets: Secure credential storage (Keychain/Keystore/Credential Manager)
//! - Audio: Native audio capture (CPAL on desktop, Web Audio fallback on mobile)
//! - TTS: Text-to-speech (future)

pub mod audio;
pub mod secrets;

pub use audio::{AudioCapture, AudioCaptureError, AudioChunk, TARGET_SAMPLE_RATE, VAD_THRESHOLD};
pub use secrets::SecureStorage;

// Re-export platform-specific implementations
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub use audio::DesktopAudioCapture;

#[cfg(any(target_os = "ios", target_os = "android"))]
pub use audio::MobileAudioCapture;
