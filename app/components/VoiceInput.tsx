'use client';

import { useCallback } from 'react';
import { useVoiceStore } from '../store/voiceStore';

export function VoiceInput() {
  const {
    recordingState,
    setRecordingState,
    isSpeechDetected,
    audioEnergy,
    setError,
  } = useVoiceStore();

  const isRecording = recordingState === 'recording';
  const isProcessing = recordingState === 'processing';

  const toggleRecording = useCallback(async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');

      if (isRecording) {
        // Stop recording and Deepgram stream
        await invoke('stop_recording');
        await invoke('stop_deepgram_stream').catch(() => {});
      } else {
        setRecordingState('recording');

        // Start Deepgram stream first (if API key exists)
        try {
          const apiKey = await invoke<string | null>('get_api_key', { keyType: 'deepgram' });
          if (apiKey) {
            console.log('[VoiceInput] Starting Deepgram stream...');
            await invoke('start_deepgram_stream', { apiKey });
            console.log('[VoiceInput] Deepgram stream started');
          } else {
            console.log('[VoiceInput] No Deepgram API key, skipping transcription');
          }
        } catch (err) {
          console.error('[VoiceInput] Failed to start Deepgram:', err);
        }

        // Then start audio recording
        await invoke('start_recording');
      }
    } catch (error) {
      console.error('Recording error:', error);
      setError(error instanceof Error ? error.message : 'Recording failed');
      setRecordingState('idle');
    }
  }, [isRecording, setRecordingState, setError]);

  // Generate waveform bars based on audio energy
  const waveformBars = Array.from({ length: 5 }, (_, i) => {
    const baseHeight = 8;
    const maxHeight = 32;
    const energyFactor = isRecording ? audioEnergy * 100 : 0;
    const variance = Math.sin((i + Date.now() / 200) * 0.5) * 0.3 + 0.7;
    const height = Math.min(
      maxHeight,
      baseHeight + energyFactor * variance
    );
    return height;
  });

  return (
    <div className="flex flex-col items-center gap-4">
      {/* Waveform visualization */}
      <div className="flex items-center justify-center gap-1 h-10">
        {isRecording ? (
          waveformBars.map((height, i) => (
            <div
              key={i}
              className={`w-1 rounded-full transition-all duration-100 ${
                isSpeechDetected ? 'bg-voice-primary' : 'bg-voice-border'
              }`}
              style={{ height: `${height}px` }}
            />
          ))
        ) : (
          <div className="text-gray-400 text-sm">
            {isProcessing ? 'Processing...' : 'Click to start recording'}
          </div>
        )}
      </div>

      {/* Record button */}
      <button
        onClick={toggleRecording}
        disabled={isProcessing}
        className={`
          relative w-16 h-16 rounded-full
          flex items-center justify-center
          transition-all duration-300 ease-out
          focus:outline-none focus-visible:ring-2 focus-visible:ring-voice-primary
          ${
            isRecording
              ? 'bg-red-500 hover:bg-red-600 scale-110'
              : isProcessing
              ? 'bg-voice-border cursor-not-allowed'
              : 'bg-voice-primary hover:bg-voice-secondary hover:scale-105'
          }
        `}
        aria-label={isRecording ? 'Stop recording' : 'Start recording'}
      >
        {/* Pulse ring when recording */}
        {isRecording && (
          <span className="absolute inset-0 rounded-full bg-red-500 animate-ping opacity-30" />
        )}

        {/* Icon */}
        {isRecording ? (
          <StopIcon className="w-6 h-6 text-white" />
        ) : isProcessing ? (
          <LoadingSpinner className="w-6 h-6 text-gray-400" />
        ) : (
          <MicrophoneIcon className="w-6 h-6 text-white" />
        )}
      </button>

      {/* Status text */}
      <div className="text-xs text-gray-500">
        {isRecording && isSpeechDetected && 'Speech detected'}
        {isRecording && !isSpeechDetected && 'Listening...'}
        {isProcessing && 'Finalizing transcript...'}
      </div>
    </div>
  );
}

function MicrophoneIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"
      />
    </svg>
  );
}

function StopIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="currentColor"
      viewBox="0 0 24 24"
    >
      <rect x="6" y="6" width="12" height="12" rx="1" />
    </svg>
  );
}

function LoadingSpinner({ className }: { className?: string }) {
  return (
    <svg
      className={`${className} animate-spin`}
      fill="none"
      viewBox="0 0 24 24"
    >
      <circle
        className="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        strokeWidth="4"
      />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      />
    </svg>
  );
}
