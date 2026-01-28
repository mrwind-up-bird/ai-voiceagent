import { create } from 'zustand';

export interface ActionItem {
  task: string;
  assignee: string | null;
  due_date: string | null;
  priority: 'high' | 'medium' | 'low';
  context: string | null;
}

export interface ToneShiftResult {
  original: string;
  shifted: string;
  tone: string;
}

export interface Track {
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
}

export interface MoodAnalysis {
  detected_mood: string;
  energy_level: number;
  valence: number;
  keywords: string[];
}

export interface TranslationResult {
  original: string;
  translated: string;
  source_language: string;
  target_language: string;
  detected_language: string | null;
}

export interface LanguageOption {
  code: string;
  name: string;
  isSource: boolean;
}

export type AgentType = 'action-items' | 'tone-shifter' | 'music-matcher' | 'translator' | null;
export type RecordingState = 'idle' | 'recording' | 'processing';

interface VoiceState {
  // Recording state
  recordingState: RecordingState;
  setRecordingState: (state: RecordingState) => void;
  hasRecording: boolean;
  recordingDuration: number;
  setHasRecording: (has: boolean) => void;
  setRecordingDuration: (duration: number) => void;

  // Transcript
  transcript: string;
  interimTranscript: string;
  setTranscript: (text: string) => void;
  setInterimTranscript: (text: string) => void;
  appendTranscript: (text: string, isFinal: boolean) => void;
  clearTranscript: () => void;

  // VAD
  isSpeechDetected: boolean;
  audioEnergy: number;
  setVadState: (isSpeech: boolean, energy: number) => void;

  // Active agent
  activeAgent: AgentType;
  setActiveAgent: (agent: AgentType) => void;

  // Agent results
  actionItems: ActionItem[];
  setActionItems: (items: ActionItem[]) => void;

  toneShiftResult: ToneShiftResult | null;
  toneShiftStreaming: string;
  setToneShiftResult: (result: ToneShiftResult | null) => void;
  appendToneShiftStreaming: (text: string) => void;
  clearToneShiftStreaming: () => void;

  musicTracks: Track[];
  moodAnalysis: MoodAnalysis | null;
  setMusicTracks: (tracks: Track[]) => void;
  setMoodAnalysis: (analysis: MoodAnalysis | null) => void;

  // Translation
  translationResult: TranslationResult | null;
  translationStreaming: string;
  selectedSourceLanguage: string;
  selectedTargetLanguage: string;
  setTranslationResult: (result: TranslationResult | null) => void;
  appendTranslationStreaming: (text: string) => void;
  clearTranslationStreaming: () => void;
  setSelectedSourceLanguage: (lang: string) => void;
  setSelectedTargetLanguage: (lang: string) => void;

  // Processing state
  isProcessing: boolean;
  processingMessage: string;
  setProcessing: (isProcessing: boolean, message?: string) => void;

  // Error handling
  error: string | null;
  setError: (error: string | null) => void;

  // Settings
  selectedTone: string;
  setSelectedTone: (tone: string) => void;
  toneIntensity: number;
  setToneIntensity: (intensity: number) => void;
  toneLengthAdjustment: number;
  setToneLengthAdjustment: (adjustment: number) => void;

  // Reset
  reset: () => void;
}

const initialState = {
  recordingState: 'idle' as RecordingState,
  hasRecording: false,
  recordingDuration: 0,
  transcript: '',
  interimTranscript: '',
  isSpeechDetected: false,
  audioEnergy: 0,
  activeAgent: null as AgentType,
  actionItems: [],
  toneShiftResult: null,
  toneShiftStreaming: '',
  musicTracks: [],
  moodAnalysis: null,
  translationResult: null,
  translationStreaming: '',
  selectedSourceLanguage: 'auto',
  selectedTargetLanguage: 'en',
  isProcessing: false,
  processingMessage: '',
  error: null,
  selectedTone: 'professional',
  toneIntensity: 5,
  toneLengthAdjustment: 0,
};

export const useVoiceStore = create<VoiceState>((set, get) => ({
  ...initialState,

  setRecordingState: (state) => set({ recordingState: state }),

  setHasRecording: (has) => set({ hasRecording: has }),

  setRecordingDuration: (duration) => set({ recordingDuration: duration }),

  setTranscript: (text) => set({ transcript: text }),

  setInterimTranscript: (text) => set({ interimTranscript: text }),

  appendTranscript: (text, isFinal) => {
    if (isFinal) {
      set((state) => ({
        transcript: state.transcript + (state.transcript ? ' ' : '') + text,
        interimTranscript: '',
      }));
    } else {
      set({ interimTranscript: text });
    }
  },

  clearTranscript: () => set({ transcript: '', interimTranscript: '' }),

  setVadState: (isSpeech, energy) =>
    set({ isSpeechDetected: isSpeech, audioEnergy: energy }),

  setActiveAgent: (agent) => set({ activeAgent: agent }),

  setActionItems: (items) => set({ actionItems: items }),

  setToneShiftResult: (result) => set({ toneShiftResult: result }),

  appendToneShiftStreaming: (text) =>
    set((state) => ({ toneShiftStreaming: state.toneShiftStreaming + text })),

  clearToneShiftStreaming: () => set({ toneShiftStreaming: '' }),

  setMusicTracks: (tracks) => set({ musicTracks: tracks }),

  setMoodAnalysis: (analysis) => set({ moodAnalysis: analysis }),

  setTranslationResult: (result) => set({ translationResult: result }),

  appendTranslationStreaming: (text) =>
    set((state) => ({ translationStreaming: state.translationStreaming + text })),

  clearTranslationStreaming: () => set({ translationStreaming: '' }),

  setSelectedSourceLanguage: (lang) => set({ selectedSourceLanguage: lang }),

  setSelectedTargetLanguage: (lang) => set({ selectedTargetLanguage: lang }),

  setProcessing: (isProcessing, message = '') =>
    set({ isProcessing, processingMessage: message }),

  setError: (error) => set({ error }),

  setSelectedTone: (tone) => set({ selectedTone: tone }),

  setToneIntensity: (intensity) => set({ toneIntensity: Math.max(1, Math.min(10, intensity)) }),

  setToneLengthAdjustment: (adjustment) => set({ toneLengthAdjustment: Math.max(-50, Math.min(100, adjustment)) }),

  reset: () => set(initialState),
}));
