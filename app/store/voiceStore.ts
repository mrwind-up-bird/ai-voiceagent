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

export type AgentType = 'action-items' | 'tone-shifter' | 'music-matcher' | null;
export type RecordingState = 'idle' | 'recording' | 'processing';

interface VoiceState {
  // Recording state
  recordingState: RecordingState;
  setRecordingState: (state: RecordingState) => void;

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

  // Reset
  reset: () => void;
}

const initialState = {
  recordingState: 'idle' as RecordingState,
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
  isProcessing: false,
  processingMessage: '',
  error: null,
  selectedTone: 'professional',
};

export const useVoiceStore = create<VoiceState>((set, get) => ({
  ...initialState,

  setRecordingState: (state) => set({ recordingState: state }),

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

  setProcessing: (isProcessing, message = '') =>
    set({ isProcessing, processingMessage: message }),

  setError: (error) => set({ error }),

  setSelectedTone: (tone) => set({ selectedTone: tone }),

  reset: () => set(initialState),
}));
