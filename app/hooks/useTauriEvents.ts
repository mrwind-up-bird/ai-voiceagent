'use client';

import { useEffect } from 'react';
import { useVoiceStore } from '../store/voiceStore';

interface TauriEvent<T> {
  payload: T;
}

interface TranscriptPayload {
  text: string;
  is_final: boolean;
  confidence: number;
  source: string;
}

interface VadPayload {
  is_speech: boolean;
  energy: number;
}

interface ActionItemsPayload {
  items: Array<{
    task: string;
    assignee: string | null;
    due_date: string | null;
    priority: 'high' | 'medium' | 'low';
    context: string | null;
  }>;
  summary: string;
}

interface ToneShiftChunkPayload {
  text: string;
  is_complete: boolean;
}

interface ToneShiftResultPayload {
  original: string;
  shifted: string;
  tone: string;
}

interface MusicMatchPayload {
  tracks: Array<{
    id: string;
    title: string;
    artist: string;
    album: string | null;
    duration_ms: number;
    preview_url: string | null;
    cover_art_url: string | null;
    match_score: number;
    mood_tags: string[];
    genre_tags: string[];
  }>;
  analysis: {
    detected_mood: string;
    energy_level: number;
    valence: number;
    keywords: string[];
  };
}

export function useTauriEvents() {
  const {
    setRecordingState,
    appendTranscript,
    setVadState,
    setActionItems,
    setToneShiftResult,
    appendToneShiftStreaming,
    clearToneShiftStreaming,
    setMusicTracks,
    setMoodAnalysis,
    setProcessing,
    setError,
  } = useVoiceStore();

  useEffect(() => {
    let listeners: Array<() => void> = [];

    async function setupListeners() {
      try {
        const { listen } = await import('@tauri-apps/api/event');

        // Recording events
        const unlistenRecordingStarted = await listen('recording-started', () => {
          setRecordingState('recording');
        });
        listeners.push(unlistenRecordingStarted);

        const unlistenRecordingStopped = await listen('recording-stopped', () => {
          // With streaming transcription, go back to idle when recording stops
          // Transcripts come in real-time during recording
          setRecordingState('idle');
        });
        listeners.push(unlistenRecordingStopped);

        // Transcript events
        const unlistenTranscript = await listen<TranscriptPayload>(
          'transcript',
          (event: TauriEvent<TranscriptPayload>) => {
            appendTranscript(event.payload.text, event.payload.is_final);
          }
        );
        listeners.push(unlistenTranscript);

        // VAD events
        const unlistenVad = await listen<VadPayload>(
          'vad-event',
          (event: TauriEvent<VadPayload>) => {
            setVadState(event.payload.is_speech, event.payload.energy);
          }
        );
        listeners.push(unlistenVad);

        // Silence detection
        const unlistenSilence = await listen('silence-detected', () => {
          // Optionally auto-stop recording on extended silence
          console.log('Silence detected');
        });
        listeners.push(unlistenSilence);

        // Action items events
        const unlistenActionItemsProcessing = await listen(
          'action-items-processing',
          () => {
            setProcessing(true, 'Extracting action items...');
          }
        );
        listeners.push(unlistenActionItemsProcessing);

        const unlistenActionItems = await listen<ActionItemsPayload>(
          'action-items-extracted',
          (event: TauriEvent<ActionItemsPayload>) => {
            setActionItems(event.payload.items);
            setProcessing(false);
          }
        );
        listeners.push(unlistenActionItems);

        // Tone shift events
        const unlistenToneShiftStarted = await listen('tone-shift-started', () => {
          clearToneShiftStreaming();
          setProcessing(true, 'Shifting tone...');
        });
        listeners.push(unlistenToneShiftStarted);

        const unlistenToneShiftChunk = await listen<ToneShiftChunkPayload>(
          'tone-shift-chunk',
          (event: TauriEvent<ToneShiftChunkPayload>) => {
            if (!event.payload.is_complete) {
              appendToneShiftStreaming(event.payload.text);
            }
          }
        );
        listeners.push(unlistenToneShiftChunk);

        const unlistenToneShiftComplete = await listen<ToneShiftResultPayload>(
          'tone-shift-complete',
          (event: TauriEvent<ToneShiftResultPayload>) => {
            setToneShiftResult(event.payload);
            setProcessing(false);
          }
        );
        listeners.push(unlistenToneShiftComplete);

        // Music match events
        const unlistenMusicMatched = await listen<MusicMatchPayload>(
          'music-matched',
          (event: TauriEvent<MusicMatchPayload>) => {
            setMusicTracks(event.payload.tracks);
            setMoodAnalysis(event.payload.analysis);
            setProcessing(false);
          }
        );
        listeners.push(unlistenMusicMatched);

        // Mood analysis event
        const unlistenMoodAnalyzed = await listen<{
          detected_mood: string;
          energy_level: number;
          valence: number;
          keywords: string[];
        }>('mood-analyzed', (event) => {
          setMoodAnalysis(event.payload);
        });
        listeners.push(unlistenMoodAnalyzed);

        // Deepgram connection
        const unlistenDeepgramConnected = await listen('deepgram-connected', () => {
          console.log('Deepgram WebSocket connected');
        });
        listeners.push(unlistenDeepgramConnected);
      } catch (error) {
        // Running outside Tauri (e.g., in browser dev mode)
        console.log('Tauri events not available:', error);
      }
    }

    setupListeners();

    return () => {
      listeners.forEach((unlisten) => unlisten());
    };
  }, [
    setRecordingState,
    appendTranscript,
    setVadState,
    setActionItems,
    setToneShiftResult,
    appendToneShiftStreaming,
    clearToneShiftStreaming,
    setMusicTracks,
    setMoodAnalysis,
    setProcessing,
    setError,
  ]);
}
