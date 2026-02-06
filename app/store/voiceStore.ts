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

export interface DevLogResult {
  commit_message: string;
  ticket: {
    title: string;
    description: string;
    acceptance_criteria: string[];
  };
  slack_update: string;
}

export type EisenhowerQuadrant = 'urgent_important' | 'not_urgent_important' | 'urgent_not_important' | 'not_urgent_not_important';

export interface BrainDumpTask {
  title: string;
  description: string;
  quadrant: EisenhowerQuadrant;
  due_hint: string | null;
}

export interface BrainDumpIdea {
  title: string;
  description: string;
  category: string | null;
  potential: string | null;
}

export interface BrainDumpNote {
  content: string;
  tags: string[];
}

export interface BrainDumpResult {
  tasks: BrainDumpTask[];
  creative_ideas: BrainDumpIdea[];
  notes: BrainDumpNote[];
  summary: string;
}

export interface MentalMirrorResult {
  reflection: string;
  mental_checkin: string;
  the_release: string;
  message_to_tomorrow: string;
  date: string;
  disclaimer: string;
}

export type AgentType = 'action-items' | 'tone-shifter' | 'music-matcher' | 'translator' | 'dev-log' | 'brain-dump' | 'mental-mirror' | null;
export type RecordingState = 'idle' | 'recording' | 'processing';
export type SyncStatus = 'disconnected' | 'waiting_for_peer' | 'connecting' | 'connected';

export interface PeerInfo {
  device_id: string;
  device_name: string;
  connected_at: number;
}

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

  // Dev-Log
  devLogResult: DevLogResult | null;
  devLogStreaming: string;
  setDevLogResult: (result: DevLogResult | null) => void;
  appendDevLogStreaming: (text: string) => void;
  clearDevLogStreaming: () => void;

  // Brain Dump
  brainDumpResult: BrainDumpResult | null;
  brainDumpStreaming: string;
  setBrainDumpResult: (result: BrainDumpResult | null) => void;
  appendBrainDumpStreaming: (text: string) => void;
  clearBrainDumpStreaming: () => void;

  // Mental Mirror (Letter to Myself)
  mentalMirrorResult: MentalMirrorResult | null;
  mentalMirrorStreaming: string;
  setMentalMirrorResult: (result: MentalMirrorResult | null) => void;
  appendMentalMirrorStreaming: (text: string) => void;
  clearMentalMirrorStreaming: () => void;

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

  // Sync
  syncStatus: SyncStatus;
  pairingCode: string | null;
  pairedDeviceName: string | null;
  syncPeer: PeerInfo | null;
  syncWarning: string | null;
  setSyncStatus: (status: SyncStatus) => void;
  setPairingCode: (code: string | null) => void;
  setPairedDeviceName: (name: string | null) => void;
  setSyncPeer: (peer: PeerInfo | null) => void;
  setSyncWarning: (warning: string | null) => void;

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
  devLogResult: null,
  devLogStreaming: '',
  brainDumpResult: null,
  brainDumpStreaming: '',
  mentalMirrorResult: null,
  mentalMirrorStreaming: '',
  isProcessing: false,
  processingMessage: '',
  error: null,
  selectedTone: 'professional',
  toneIntensity: 5,
  toneLengthAdjustment: 0,
  syncStatus: 'disconnected' as SyncStatus,
  pairingCode: null,
  pairedDeviceName: null,
  syncPeer: null,
  syncWarning: null,
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

  setDevLogResult: (result) => set({ devLogResult: result }),

  appendDevLogStreaming: (text) =>
    set((state) => ({ devLogStreaming: state.devLogStreaming + text })),

  clearDevLogStreaming: () => set({ devLogStreaming: '' }),

  setBrainDumpResult: (result) => set({ brainDumpResult: result }),

  appendBrainDumpStreaming: (text) =>
    set((state) => ({ brainDumpStreaming: state.brainDumpStreaming + text })),

  clearBrainDumpStreaming: () => set({ brainDumpStreaming: '' }),

  setMentalMirrorResult: (result) => set({ mentalMirrorResult: result }),

  appendMentalMirrorStreaming: (text) =>
    set((state) => ({ mentalMirrorStreaming: state.mentalMirrorStreaming + text })),

  clearMentalMirrorStreaming: () => set({ mentalMirrorStreaming: '' }),

  setProcessing: (isProcessing, message = '') =>
    set({ isProcessing, processingMessage: message }),

  setError: (error) => set({ error }),

  setSelectedTone: (tone) => set({ selectedTone: tone }),

  setToneIntensity: (intensity) => set({ toneIntensity: Math.max(1, Math.min(10, intensity)) }),

  setToneLengthAdjustment: (adjustment) => set({ toneLengthAdjustment: Math.max(-50, Math.min(100, adjustment)) }),

  setSyncStatus: (status) => set({ syncStatus: status }),
  setPairingCode: (code) => set({ pairingCode: code }),
  setPairedDeviceName: (name) => set({ pairedDeviceName: name }),
  setSyncPeer: (peer) => set({ syncPeer: peer }),
  setSyncWarning: (warning) => set({ syncWarning: warning }),

  reset: () => set(initialState),
}));
