/**
 * Web Audio API capture hook for mobile platforms
 *
 * On desktop, audio capture is handled natively by Rust via CPAL.
 * On mobile (iOS/Android), we use Web Audio API in the WebView and
 * send samples to Rust via Tauri commands for Deepgram streaming.
 *
 * This hook provides:
 * - MediaStream access with microphone permission handling
 * - AudioWorklet-based sample processing at 16kHz mono
 * - Automatic forwarding to Rust backend via send_audio_to_deepgram
 */

import { useCallback, useRef, useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { usePlatform } from './usePlatform';

const TARGET_SAMPLE_RATE = 16000;
const CHUNK_SIZE = 1600; // 100ms at 16kHz

interface WebAudioCaptureState {
  isRecording: boolean;
  isSupported: boolean;
  error: string | null;
  permissionState: PermissionState | null;
}

interface WebAudioCaptureResult extends WebAudioCaptureState {
  startRecording: () => Promise<void>;
  stopRecording: () => void;
  requestPermission: () => Promise<boolean>;
}

/**
 * Hook for capturing audio via Web Audio API on mobile platforms.
 * Automatically sends audio samples to Rust backend for Deepgram transcription.
 */
export function useWebAudioCapture(): WebAudioCaptureResult {
  const { isMobile, isDesktop } = usePlatform();
  const [state, setState] = useState<WebAudioCaptureState>({
    isRecording: false,
    isSupported: false,
    error: null,
    permissionState: null,
  });

  const mediaStreamRef = useRef<MediaStream | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const workletNodeRef = useRef<AudioWorkletNode | null>(null);
  const bufferRef = useRef<number[]>([]);

  // Check if Web Audio is supported
  useEffect(() => {
    const isSupported =
      typeof window !== 'undefined' &&
      'AudioContext' in window &&
      'mediaDevices' in navigator &&
      'getUserMedia' in navigator.mediaDevices;

    setState((prev) => ({ ...prev, isSupported }));

    // Check permission state if available
    if (isSupported && navigator.permissions) {
      navigator.permissions
        .query({ name: 'microphone' as PermissionName })
        .then((result) => {
          setState((prev) => ({ ...prev, permissionState: result.state }));
          result.onchange = () => {
            setState((prev) => ({ ...prev, permissionState: result.state }));
          };
        })
        .catch(() => {
          // Permission API not supported for microphone, that's ok
        });
    }
  }, []);

  /**
   * Request microphone permission
   */
  const requestPermission = useCallback(async (): Promise<boolean> => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      // Stop all tracks immediately - we just wanted to trigger permission prompt
      stream.getTracks().forEach((track) => track.stop());
      setState((prev) => ({ ...prev, permissionState: 'granted', error: null }));
      return true;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Permission denied';
      setState((prev) => ({
        ...prev,
        permissionState: 'denied',
        error: `Microphone permission denied: ${message}`,
      }));
      return false;
    }
  }, []);

  /**
   * Process audio samples and send to Rust backend
   */
  const processAudioData = useCallback(async (samples: Float32Array) => {
    // Convert Float32 (-1 to 1) to Int16 (-32768 to 32767)
    const int16Samples = new Int16Array(samples.length);
    for (let i = 0; i < samples.length; i++) {
      int16Samples[i] = Math.max(-32768, Math.min(32767, Math.round(samples[i] * 32767)));
    }

    // Buffer samples until we have a chunk
    bufferRef.current.push(...Array.from(int16Samples));

    // Send chunks of 100ms
    while (bufferRef.current.length >= CHUNK_SIZE) {
      const chunk = bufferRef.current.splice(0, CHUNK_SIZE);
      try {
        await invoke('send_audio_to_deepgram', { samples: chunk });
      } catch (error) {
        console.error('Failed to send audio to Deepgram:', error);
      }
    }
  }, []);

  /**
   * Start audio capture
   */
  const startRecording = useCallback(async () => {
    // On desktop, audio is handled by Rust - this hook is for mobile only
    if (isDesktop) {
      setState((prev) => ({
        ...prev,
        error: 'Use native audio capture on desktop',
      }));
      return;
    }

    if (!state.isSupported) {
      setState((prev) => ({
        ...prev,
        error: 'Web Audio API not supported',
      }));
      return;
    }

    try {
      // Request microphone access
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          sampleRate: TARGET_SAMPLE_RATE,
          channelCount: 1,
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });

      mediaStreamRef.current = stream;

      // Create audio context at target sample rate
      const audioContext = new AudioContext({
        sampleRate: TARGET_SAMPLE_RATE,
      });
      audioContextRef.current = audioContext;

      // Create source from microphone stream
      const source = audioContext.createMediaStreamSource(stream);

      // Use ScriptProcessor for simple sample access (AudioWorklet requires more setup)
      // Note: ScriptProcessor is deprecated but still widely supported
      const processor = audioContext.createScriptProcessor(4096, 1, 1);

      processor.onaudioprocess = (event) => {
        const inputData = event.inputBuffer.getChannelData(0);
        processAudioData(new Float32Array(inputData));
      };

      source.connect(processor);
      processor.connect(audioContext.destination);

      // Store processor reference for cleanup
      (audioContextRef.current as AudioContext & { processor?: ScriptProcessorNode }).processor =
        processor;

      setState((prev) => ({
        ...prev,
        isRecording: true,
        error: null,
        permissionState: 'granted',
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to start recording';
      setState((prev) => ({
        ...prev,
        isRecording: false,
        error: message,
      }));
    }
  }, [isDesktop, state.isSupported, processAudioData]);

  /**
   * Stop audio capture
   */
  const stopRecording = useCallback(() => {
    // Stop media stream tracks
    if (mediaStreamRef.current) {
      mediaStreamRef.current.getTracks().forEach((track) => track.stop());
      mediaStreamRef.current = null;
    }

    // Close audio context
    if (audioContextRef.current) {
      const ctx = audioContextRef.current as AudioContext & { processor?: ScriptProcessorNode };
      if (ctx.processor) {
        ctx.processor.disconnect();
      }
      audioContextRef.current.close();
      audioContextRef.current = null;
    }

    // Clear worklet node
    if (workletNodeRef.current) {
      workletNodeRef.current.disconnect();
      workletNodeRef.current = null;
    }

    // Clear buffer
    bufferRef.current = [];

    setState((prev) => ({
      ...prev,
      isRecording: false,
    }));
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopRecording();
    };
  }, [stopRecording]);

  return {
    ...state,
    startRecording,
    stopRecording,
    requestPermission,
  };
}
