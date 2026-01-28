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

interface TranslationChunkPayload {
  text: string;
  is_complete: boolean;
}

interface TranslationResultPayload {
  original: string;
  translated: string;
  source_language: string;
  target_language: string;
  detected_language: string | null;
}

interface DevLogChunkPayload {
  text: string;
  is_complete: boolean;
}

interface DevLogResultPayload {
  commit_message: string;
  ticket: {
    title: string;
    description: string;
    acceptance_criteria: string[];
  };
  slack_update: string;
}

interface BrainDumpChunkPayload {
  text: string;
  is_complete: boolean;
}

interface BrainDumpResultPayload {
  tasks: Array<{
    title: string;
    description: string;
    quadrant: 'urgent_important' | 'not_urgent_important' | 'urgent_not_important' | 'not_urgent_not_important';
    due_hint: string | null;
  }>;
  creative_ideas: Array<{
    title: string;
    description: string;
    category: string | null;
    potential: string | null;
  }>;
  notes: Array<{
    content: string;
    tags: string[];
  }>;
  summary: string;
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
    setHasRecording,
    setRecordingDuration,
    appendTranscript,
    setVadState,
    setActionItems,
    setToneShiftResult,
    appendToneShiftStreaming,
    clearToneShiftStreaming,
    setMusicTracks,
    setMoodAnalysis,
    setTranslationResult,
    appendTranslationStreaming,
    clearTranslationStreaming,
    setDevLogResult,
    appendDevLogStreaming,
    clearDevLogStreaming,
    setBrainDumpResult,
    appendBrainDumpStreaming,
    clearBrainDumpStreaming,
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

        const unlistenRecordingStopped = await listen('recording-stopped', async () => {
          // With streaming transcription, go back to idle when recording stops
          // Transcripts come in real-time during recording
          setRecordingState('idle');

          // Check if there's recorded audio available
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            const hasRec = await invoke<boolean>('has_recording');
            setHasRecording(hasRec);
            if (hasRec) {
              const duration = await invoke<number>('get_recording_duration');
              setRecordingDuration(duration);
            }
          } catch {
            // Ignore errors
          }
        });
        listeners.push(unlistenRecordingStopped);

        const unlistenRecordingSaved = await listen<{ filepath: string; duration_secs: number }>(
          'recording-saved',
          (event) => {
            console.log('Recording saved:', event.payload.filepath);
          }
        );
        listeners.push(unlistenRecordingSaved);

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

        // Translation events
        const unlistenTranslationStarted = await listen('translation-started', () => {
          clearTranslationStreaming();
          setProcessing(true, 'Translating...');
        });
        listeners.push(unlistenTranslationStarted);

        const unlistenTranslationChunk = await listen<TranslationChunkPayload>(
          'translation-chunk',
          (event: TauriEvent<TranslationChunkPayload>) => {
            if (!event.payload.is_complete) {
              appendTranslationStreaming(event.payload.text);
            }
          }
        );
        listeners.push(unlistenTranslationChunk);

        const unlistenTranslationComplete = await listen<TranslationResultPayload>(
          'translation-complete',
          (event: TauriEvent<TranslationResultPayload>) => {
            setTranslationResult(event.payload);
            setProcessing(false);
          }
        );
        listeners.push(unlistenTranslationComplete);

        // Dev-Log events
        const unlistenDevLogStarted = await listen('dev-log-started', () => {
          clearDevLogStreaming();
          setProcessing(true, 'Generating dev documentation...');
        });
        listeners.push(unlistenDevLogStarted);

        const unlistenDevLogChunk = await listen<DevLogChunkPayload>(
          'dev-log-chunk',
          (event: TauriEvent<DevLogChunkPayload>) => {
            if (!event.payload.is_complete) {
              appendDevLogStreaming(event.payload.text);
            }
          }
        );
        listeners.push(unlistenDevLogChunk);

        const unlistenDevLogComplete = await listen<DevLogResultPayload>(
          'dev-log-complete',
          (event: TauriEvent<DevLogResultPayload>) => {
            setDevLogResult(event.payload);
            setProcessing(false);
          }
        );
        listeners.push(unlistenDevLogComplete);

        // Brain Dump events
        const unlistenBrainDumpStarted = await listen('brain-dump-started', () => {
          clearBrainDumpStreaming();
          setProcessing(true, 'Processing brain dump...');
        });
        listeners.push(unlistenBrainDumpStarted);

        const unlistenBrainDumpChunk = await listen<BrainDumpChunkPayload>(
          'brain-dump-chunk',
          (event: TauriEvent<BrainDumpChunkPayload>) => {
            if (!event.payload.is_complete) {
              appendBrainDumpStreaming(event.payload.text);
            }
          }
        );
        listeners.push(unlistenBrainDumpChunk);

        const unlistenBrainDumpComplete = await listen<BrainDumpResultPayload>(
          'brain-dump-complete',
          (event: TauriEvent<BrainDumpResultPayload>) => {
            setBrainDumpResult(event.payload);
            setProcessing(false);
          }
        );
        listeners.push(unlistenBrainDumpComplete);
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
    setHasRecording,
    setRecordingDuration,
    appendTranscript,
    setVadState,
    setActionItems,
    setToneShiftResult,
    appendToneShiftStreaming,
    clearToneShiftStreaming,
    setMusicTracks,
    setMoodAnalysis,
    setTranslationResult,
    appendTranslationStreaming,
    clearTranslationStreaming,
    setDevLogResult,
    appendDevLogStreaming,
    clearDevLogStreaming,
    setBrainDumpResult,
    appendBrainDumpStreaming,
    clearBrainDumpStreaming,
    setProcessing,
    setError,
  ]);
}
