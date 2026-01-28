'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import { useVoiceStore } from '../store/voiceStore';

interface TonePreset {
  id: string;
  name: string;
  description: string;
  use_cases: string[];
  example_before: string;
  example_after: string;
  icon: string;
  color: string;
}

// Debounce hook for slider
function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedValue(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);

  return debouncedValue;
}

export function ToneSelector() {
  const {
    activeAgent,
    selectedTone,
    setSelectedTone,
    toneIntensity,
    setToneIntensity,
    toneLengthAdjustment,
    setToneLengthAdjustment,
    transcript,
    toneShiftResult,
    isProcessing,
    setProcessing,
    setError,
    clearToneShiftStreaming,
    setToneShiftResult,
  } = useVoiceStore();

  const [presets, setPresets] = useState<TonePreset[]>([]);
  const [expandedPreset, setExpandedPreset] = useState<string | null>(null);

  // Track if user has triggered tone shift at least once
  const hasRunOnce = useRef(false);
  const previousTone = useRef(selectedTone);
  const previousIntensity = useRef(toneIntensity);
  const previousLengthAdjustment = useRef(toneLengthAdjustment);

  // Debounce slider changes (500ms delay)
  const debouncedIntensity = useDebounce(toneIntensity, 500);
  const debouncedLengthAdjustment = useDebounce(toneLengthAdjustment, 500);

  const getIntensityLabel = (value: number): string => {
    if (value <= 3) return 'Subtle';
    if (value <= 6) return 'Moderate';
    if (value <= 8) return 'Strong';
    return 'Maximum';
  };

  const getLengthLabel = (value: number): string => {
    if (value <= -30) return 'Much Shorter';
    if (value <= -10) return 'Shorter';
    if (value < 10) return 'Same Length';
    if (value < 30) return 'Longer';
    if (value < 60) return 'Much Longer';
    return 'Very Long';
  };

  // Run tone shift
  const runToneShift = useCallback(async (tone: string, intensity: number, lengthAdjustment: number) => {
    if (!transcript || isProcessing) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const anthropicKey = await invoke<string | null>('get_api_key', { keyType: 'anthropic' });

      if (!anthropicKey) {
        setError('Anthropic API key required for Tone Shifter');
        return;
      }

      // Clear previous results before starting new request
      clearToneShiftStreaming();
      setToneShiftResult(null);
      setProcessing(true, 'Shifting tone...');

      await invoke('shift_tone_streaming', {
        apiKey: anthropicKey,
        text: transcript,
        targetTone: tone,
        intensity: intensity,
        lengthAdjustment: lengthAdjustment,
      });
    } catch (error) {
      console.error('Tone shift error:', error);
      setError(error instanceof Error ? error.message : 'Tone shift failed');
      setProcessing(false);
    }
  }, [transcript, isProcessing, setProcessing, setError, clearToneShiftStreaming, setToneShiftResult]);

  // Auto-rerun when tone changes (immediate)
  useEffect(() => {
    if (
      activeAgent === 'tone-shifter' &&
      hasRunOnce.current &&
      selectedTone !== previousTone.current &&
      !isProcessing &&
      transcript
    ) {
      previousTone.current = selectedTone;
      runToneShift(selectedTone, toneIntensity, toneLengthAdjustment);
    }
  }, [selectedTone, activeAgent, isProcessing, toneIntensity, toneLengthAdjustment, runToneShift, transcript]);

  // Auto-rerun when intensity changes (debounced)
  useEffect(() => {
    if (
      activeAgent === 'tone-shifter' &&
      hasRunOnce.current &&
      debouncedIntensity !== previousIntensity.current &&
      !isProcessing &&
      transcript
    ) {
      previousIntensity.current = debouncedIntensity;
      runToneShift(selectedTone, debouncedIntensity, toneLengthAdjustment);
    }
  }, [debouncedIntensity, activeAgent, isProcessing, selectedTone, toneLengthAdjustment, runToneShift, transcript]);

  // Auto-rerun when length adjustment changes (debounced)
  useEffect(() => {
    if (
      activeAgent === 'tone-shifter' &&
      hasRunOnce.current &&
      debouncedLengthAdjustment !== previousLengthAdjustment.current &&
      !isProcessing &&
      transcript
    ) {
      previousLengthAdjustment.current = debouncedLengthAdjustment;
      runToneShift(selectedTone, toneIntensity, debouncedLengthAdjustment);
    }
  }, [debouncedLengthAdjustment, activeAgent, isProcessing, selectedTone, toneIntensity, runToneShift, transcript]);

  // Track when tone shift has been run at least once (when processing completes)
  useEffect(() => {
    if (toneShiftResult && activeAgent === 'tone-shifter' && !isProcessing) {
      // Only initialize previous values on FIRST run
      // Subsequent runs are handled by auto-rerun effects which update refs before API call
      if (!hasRunOnce.current) {
        previousTone.current = selectedTone;
        previousIntensity.current = toneIntensity;
        previousLengthAdjustment.current = toneLengthAdjustment;
      }
      hasRunOnce.current = true;
    }
  }, [toneShiftResult, activeAgent, isProcessing, selectedTone, toneIntensity, toneLengthAdjustment]);

  // Reset tracking when agent changes
  useEffect(() => {
    if (activeAgent !== 'tone-shifter') {
      hasRunOnce.current = false;
    }
  }, [activeAgent]);

  useEffect(() => {
    async function loadPresets() {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const data = await invoke<TonePreset[]>('get_tone_presets');
        setPresets(data);
      } catch (err) {
        console.error('Failed to load tone presets:', err);
        // Fallback presets for browser dev
        setPresets([
          { id: 'professional', name: 'Professional', description: 'Business-appropriate', use_cases: ['Emails', 'Reports'], example_before: '', example_after: '', icon: 'briefcase', color: '#3B82F6' },
          { id: 'casual', name: 'Casual', description: 'Relaxed and informal', use_cases: ['Messages', 'Social'], example_before: '', example_after: '', icon: 'chat', color: '#10B981' },
          { id: 'friendly', name: 'Friendly', description: 'Warm and approachable', use_cases: ['Support', 'Welcome'], example_before: '', example_after: '', icon: 'smile', color: '#F59E0B' },
          { id: 'formal', name: 'Formal', description: 'Official and structured', use_cases: ['Legal', 'Academic'], example_before: '', example_after: '', icon: 'document', color: '#6366F1' },
          { id: 'empathetic', name: 'Empathetic', description: 'Understanding and compassionate', use_cases: ['Support', 'Apologies'], example_before: '', example_after: '', icon: 'heart', color: '#EC4899' },
          { id: 'assertive', name: 'Assertive', description: 'Confident and direct', use_cases: ['Negotiations', 'Leadership'], example_before: '', example_after: '', icon: 'bolt', color: '#EF4444' },
          { id: 'diplomatic', name: 'Diplomatic', description: 'Tactful and balanced', use_cases: ['Feedback', 'Conflicts'], example_before: '', example_after: '', icon: 'scale', color: '#8B5CF6' },
          { id: 'enthusiastic', name: 'Enthusiastic', description: 'Energetic and positive', use_cases: ['Marketing', 'Announcements'], example_before: '', example_after: '', icon: 'sparkles', color: '#F97316' },
        ]);
      }
    }
    loadPresets();
  }, []);

  if (activeAgent !== 'tone-shifter') {
    return null;
  }

  return (
    <div className="w-full">
      <div className="grid grid-cols-4 gap-2">
        {presets.map((preset) => (
          <button
            key={preset.id}
            onClick={() => {
              setSelectedTone(preset.id);
              setExpandedPreset(expandedPreset === preset.id ? null : preset.id);
            }}
            className={`
              relative p-2 rounded-lg text-left transition-all duration-200
              ${selectedTone === preset.id
                ? 'bg-voice-primary/20 ring-1 ring-voice-primary'
                : 'bg-voice-surface/50 hover:bg-voice-surface'
              }
            `}
          >
            <div className="flex flex-col items-center gap-1">
              <div
                className="w-8 h-8 rounded-full flex items-center justify-center"
                style={{ backgroundColor: `${preset.color}20` }}
              >
                <ToneIcon icon={preset.icon} color={preset.color} />
              </div>
              <span className="text-xs font-medium text-white truncate w-full text-center">
                {preset.name}
              </span>
            </div>
          </button>
        ))}
      </div>

      {/* Expanded preset details */}
      {expandedPreset && (
        <div className="mt-3 p-3 bg-voice-surface/50 rounded-lg">
          {presets.filter(p => p.id === expandedPreset).map((preset) => (
            <div key={preset.id}>
              <div className="flex items-center gap-2 mb-2">
                <div
                  className="w-6 h-6 rounded-full flex items-center justify-center"
                  style={{ backgroundColor: `${preset.color}20` }}
                >
                  <ToneIcon icon={preset.icon} color={preset.color} size={14} />
                </div>
                <span className="text-sm font-medium text-white">{preset.name}</span>
              </div>
              <p className="text-xs text-gray-400 mb-2">{preset.description}</p>
              <div className="flex flex-wrap gap-1 mb-2">
                {preset.use_cases.map((useCase, i) => (
                  <span
                    key={i}
                    className="px-1.5 py-0.5 text-xs rounded"
                    style={{ backgroundColor: `${preset.color}20`, color: preset.color }}
                  >
                    {useCase}
                  </span>
                ))}
              </div>
              {preset.example_before && preset.example_after && (
                <div className="text-xs space-y-1 mt-2 pt-2 border-t border-voice-border">
                  <div className="text-gray-500">
                    <span className="text-gray-400">Before:</span> "{preset.example_before}"
                  </div>
                  <div className="text-gray-300">
                    <span className="text-gray-400">After:</span> "{preset.example_after}"
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Intensity slider */}
      <div className="mt-3 p-3 bg-voice-surface/50 rounded-lg">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-gray-400 uppercase tracking-wider">
            Intensity
          </span>
          <span className="text-xs text-white">
            {getIntensityLabel(toneIntensity)} ({toneIntensity}/10)
          </span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-gray-500">Subtle</span>
          <input
            type="range"
            min="1"
            max="10"
            value={toneIntensity}
            onChange={(e) => setToneIntensity(parseInt(e.target.value))}
            className="flex-1 h-1.5 bg-voice-border rounded-full appearance-none cursor-pointer
              [&::-webkit-slider-thumb]:appearance-none
              [&::-webkit-slider-thumb]:w-4
              [&::-webkit-slider-thumb]:h-4
              [&::-webkit-slider-thumb]:rounded-full
              [&::-webkit-slider-thumb]:bg-voice-primary
              [&::-webkit-slider-thumb]:cursor-pointer
              [&::-webkit-slider-thumb]:transition-transform
              [&::-webkit-slider-thumb]:hover:scale-110
              [&::-moz-range-thumb]:w-4
              [&::-moz-range-thumb]:h-4
              [&::-moz-range-thumb]:rounded-full
              [&::-moz-range-thumb]:bg-voice-primary
              [&::-moz-range-thumb]:border-0
              [&::-moz-range-thumb]:cursor-pointer"
          />
          <span className="text-xs text-gray-500">Max</span>
        </div>
        <div className="mt-2 text-xs text-gray-500">
          {toneIntensity <= 3 && 'Minimal changes - keeps most of the original wording'}
          {toneIntensity > 3 && toneIntensity <= 6 && 'Balanced changes - adjusts vocabulary and phrasing'}
          {toneIntensity > 6 && toneIntensity <= 8 && 'Significant rewrite - substantially different language'}
          {toneIntensity > 8 && 'Complete transformation - dramatically different style'}
        </div>
      </div>

      {/* Length adjustment slider */}
      <div className="mt-3 p-3 bg-voice-surface/50 rounded-lg">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-gray-400 uppercase tracking-wider">
            Output Length
          </span>
          <span className="text-xs text-white">
            {getLengthLabel(toneLengthAdjustment)} ({toneLengthAdjustment > 0 ? '+' : ''}{toneLengthAdjustment}%)
          </span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-gray-500">-50%</span>
          <input
            type="range"
            min="-50"
            max="100"
            step="10"
            value={toneLengthAdjustment}
            onChange={(e) => setToneLengthAdjustment(parseInt(e.target.value))}
            className="flex-1 h-1.5 bg-voice-border rounded-full appearance-none cursor-pointer
              [&::-webkit-slider-thumb]:appearance-none
              [&::-webkit-slider-thumb]:w-4
              [&::-webkit-slider-thumb]:h-4
              [&::-webkit-slider-thumb]:rounded-full
              [&::-webkit-slider-thumb]:bg-emerald-500
              [&::-webkit-slider-thumb]:cursor-pointer
              [&::-webkit-slider-thumb]:transition-transform
              [&::-webkit-slider-thumb]:hover:scale-110
              [&::-moz-range-thumb]:w-4
              [&::-moz-range-thumb]:h-4
              [&::-moz-range-thumb]:rounded-full
              [&::-moz-range-thumb]:bg-emerald-500
              [&::-moz-range-thumb]:border-0
              [&::-moz-range-thumb]:cursor-pointer"
          />
          <span className="text-xs text-gray-500">+100%</span>
        </div>
        <div className="mt-2 text-xs text-gray-500">
          {toneLengthAdjustment <= -30 && 'Very concise - removes redundancy, brief phrasing'}
          {toneLengthAdjustment > -30 && toneLengthAdjustment <= -10 && 'More concise - tightens sentences'}
          {toneLengthAdjustment > -10 && toneLengthAdjustment < 10 && 'Maintains original length'}
          {toneLengthAdjustment >= 10 && toneLengthAdjustment < 30 && 'Adds more detail and context'}
          {toneLengthAdjustment >= 30 && toneLengthAdjustment < 60 && 'Significantly expanded with examples'}
          {toneLengthAdjustment >= 60 && 'Extensive elaboration and background'}
        </div>
      </div>
    </div>
  );
}

function ToneIcon({ icon, color, size = 16 }: { icon: string; color: string; size?: number }) {
  const style = { width: size, height: size, color };

  switch (icon) {
    case 'briefcase':
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M21 13.255A23.931 23.931 0 0112 15c-3.183 0-6.22-.62-9-1.745M16 6V4a2 2 0 00-2-2h-4a2 2 0 00-2 2v2m4 6h.01M5 20h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
      );
    case 'chat':
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
        </svg>
      );
    case 'smile':
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M14.828 14.828a4 4 0 01-5.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      );
    case 'document':
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
      );
    case 'heart':
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
        </svg>
      );
    case 'bolt':
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
      );
    case 'scale':
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M3 6l3 1m0 0l-3 9a5.002 5.002 0 006.001 0M6 7l3 9M6 7l6-2m6 2l3-1m-3 1l-3 9a5.002 5.002 0 006.001 0M18 7l3 9m-3-9l-6-2m0-2v2m0 16V5m0 16H9m3 0h3" />
        </svg>
      );
    case 'sparkles':
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z" />
        </svg>
      );
    default:
      return (
        <svg style={style} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
        </svg>
      );
  }
}
