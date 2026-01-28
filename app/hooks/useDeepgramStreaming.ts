'use client';

/**
 * Hook placeholder for audio forwarding.
 *
 * Audio forwarding is now handled directly in Rust (audio.rs -> transcription.rs)
 * to avoid JSON serialization corruption of i16 samples.
 *
 * This hook is kept as a no-op for potential future use (e.g., web-only mode).
 */
export function useAudioForwarding() {
  // No-op: audio is sent directly from Rust to Deepgram
}
