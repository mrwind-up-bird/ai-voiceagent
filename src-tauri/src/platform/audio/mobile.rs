//! Mobile audio capture placeholder.
//!
//! On mobile platforms (iOS/Android), audio capture is handled differently:
//! - Web Audio API in the WebView captures audio in the frontend
//! - Audio samples are sent to Rust via Tauri commands for processing
//!
//! This module provides a stub implementation that returns NotSupported errors,
//! directing mobile apps to use the frontend-based audio capture instead.

use super::{AudioCapture, AudioCaptureError, AudioChunk};

/// Mobile audio capture stub
///
/// This implementation always returns NotSupported errors because
/// mobile audio capture is handled via Web Audio API in the frontend.
pub struct MobileAudioCapture;

impl MobileAudioCapture {
    /// Create a new mobile audio capture instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for MobileAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCapture for MobileAudioCapture {
    fn start(&self, _callback: Box<dyn Fn(AudioChunk) + Send + Sync>) -> Result<(), AudioCaptureError> {
        // On mobile, audio capture is handled by Web Audio API in the frontend
        // The frontend sends audio samples to Rust via Tauri commands
        Err(AudioCaptureError::NotSupported)
    }

    fn stop(&self) -> Result<(), AudioCaptureError> {
        Err(AudioCaptureError::NotSupported)
    }

    fn is_recording(&self) -> bool {
        false
    }

    fn list_devices(&self) -> Result<Vec<String>, AudioCaptureError> {
        // Mobile devices typically have a single microphone exposed via Web Audio
        Err(AudioCaptureError::NotSupported)
    }

    fn current_device(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobile_capture_not_supported() {
        let capture = MobileAudioCapture::new();

        // All operations should return NotSupported
        assert!(matches!(
            capture.start(Box::new(|_| {})),
            Err(AudioCaptureError::NotSupported)
        ));
        assert!(matches!(capture.stop(), Err(AudioCaptureError::NotSupported)));
        assert!(!capture.is_recording());
        assert!(matches!(
            capture.list_devices(),
            Err(AudioCaptureError::NotSupported)
        ));
        assert!(capture.current_device().is_none());
    }
}
