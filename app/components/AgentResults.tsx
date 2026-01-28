'use client';

import { useState, useCallback } from 'react';
import { useVoiceStore, AgentType, ActionItem, Track, TranslationResult } from '../store/voiceStore';

function useCopyToClipboard() {
  const [copied, setCopied] = useState(false);

  const copy = useCallback(async (text: string) => {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }, []);

  return { copied, copy };
}

function useTextToSpeech() {
  const [isSpeaking, setIsSpeaking] = useState(false);

  const speak = useCallback(async (text: string) => {
    if (!text) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      setIsSpeaking(true);
      await invoke('speak_text', { text, rate: 1.0 });
      setIsSpeaking(false);
    } catch (err) {
      console.error('TTS error:', err);
      setIsSpeaking(false);
    }
  }, []);

  const stop = useCallback(async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('stop_speech');
      setIsSpeaking(false);
    } catch (err) {
      console.error('Stop TTS error:', err);
    }
  }, []);

  return { isSpeaking, speak, stop };
}

export function AgentResults() {
  const {
    activeAgent,
    actionItems,
    toneShiftResult,
    toneShiftStreaming,
    musicTracks,
    moodAnalysis,
    translationResult,
    translationStreaming,
    isProcessing,
    processingMessage,
  } = useVoiceStore();

  if (!activeAgent) {
    return null;
  }

  return (
    <div className="w-full">
      <div className="glass rounded-lg p-4">
        {isProcessing && (
          <div className="flex items-center gap-3 text-gray-400">
            <LoadingSpinner />
            <span className="text-sm">{processingMessage || 'Processing...'}</span>
          </div>
        )}

        {activeAgent === 'action-items' && !isProcessing && (
          <ActionItemsDisplay items={actionItems} />
        )}

        {activeAgent === 'tone-shifter' && (
          <ToneShiftDisplay
            result={toneShiftResult}
            streaming={toneShiftStreaming}
            isProcessing={isProcessing}
          />
        )}

        {activeAgent === 'music-matcher' && !isProcessing && (
          <MusicMatchDisplay tracks={musicTracks} mood={moodAnalysis} />
        )}

        {activeAgent === 'translator' && (
          <TranslationDisplay
            result={translationResult}
            streaming={translationStreaming}
            isProcessing={isProcessing}
          />
        )}
      </div>
    </div>
  );
}

function ActionItemsDisplay({ items }: { items: ActionItem[] }) {
  const { copied, copy } = useCopyToClipboard();

  if (items.length === 0) {
    return (
      <div className="text-gray-400 text-sm">No action items found.</div>
    );
  }

  const formatForCopy = () => {
    return items
      .map((item, i) => {
        let line = `${i + 1}. [${item.priority.toUpperCase()}] ${item.task}`;
        if (item.assignee) line += ` (@${item.assignee})`;
        if (item.due_date) line += ` - Due: ${item.due_date}`;
        return line;
      })
      .join('\n');
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
          Action Items ({items.length})
        </h3>
        <CopyButton copied={copied} onClick={() => copy(formatForCopy())} />
      </div>
      <ul className="space-y-3">
        {items.map((item, index) => (
          <li key={index} className="flex gap-3">
            <span
              className={`
                flex-shrink-0 w-2 h-2 mt-1.5 rounded-full
                ${item.priority === 'high' ? 'bg-red-500' : ''}
                ${item.priority === 'medium' ? 'bg-yellow-500' : ''}
                ${item.priority === 'low' ? 'bg-green-500' : ''}
              `}
            />
            <div className="flex-1 min-w-0">
              <p className="text-white text-sm">{item.task}</p>
              <div className="flex flex-wrap gap-2 mt-1">
                {item.assignee && (
                  <span className="text-xs text-gray-400">
                    @{item.assignee}
                  </span>
                )}
                {item.due_date && (
                  <span className="text-xs text-gray-400">
                    Due: {item.due_date}
                  </span>
                )}
              </div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

function ToneShiftDisplay({
  result,
  streaming,
  isProcessing,
}: {
  result: { original: string; shifted: string; tone: string } | null;
  streaming: string;
  isProcessing: boolean;
}) {
  const { copied, copy } = useCopyToClipboard();
  const { isSpeaking, speak, stop } = useTextToSpeech();
  const displayText = isProcessing ? streaming : result?.shifted;

  if (!displayText && !isProcessing) {
    return null;
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
          Tone Shifted {result?.tone && `(${result.tone})`}
        </h3>
        {!isProcessing && displayText && (
          <div className="flex items-center gap-1">
            <SpeakButton isSpeaking={isSpeaking} onSpeak={() => speak(displayText)} onStop={stop} />
            <CopyButton copied={copied} onClick={() => copy(displayText)} />
          </div>
        )}
      </div>
      <div className="text-white text-sm leading-relaxed">
        {displayText}
        {isProcessing && <span className="animate-pulse">|</span>}
      </div>
    </div>
  );
}

function MusicMatchDisplay({
  tracks,
  mood,
}: {
  tracks: Track[];
  mood: { detected_mood: string; energy_level: number; valence: number; keywords: string[] } | null;
}) {
  return (
    <div>
      {mood && (
        <div className="mb-4">
          <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider mb-2">
            Mood Analysis
          </h3>
          <div className="flex flex-wrap gap-2">
            <span className="px-2 py-1 bg-voice-primary/20 text-voice-primary rounded text-xs">
              {mood.detected_mood}
            </span>
            <span className="px-2 py-1 bg-voice-surface text-gray-300 rounded text-xs">
              Energy: {Math.round(mood.energy_level * 100)}%
            </span>
            <span className="px-2 py-1 bg-voice-surface text-gray-300 rounded text-xs">
              Valence: {Math.round(mood.valence * 100)}%
            </span>
          </div>
          {mood.keywords.length > 0 && (
            <div className="flex flex-wrap gap-1 mt-2">
              {mood.keywords.map((keyword, i) => (
                <span key={i} className="text-xs text-gray-500">
                  #{keyword}
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      {tracks.length > 0 && (
        <div>
          <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider mb-3">
            Recommended Tracks ({tracks.length})
          </h3>
          <ul className="space-y-3">
            {tracks.map((track) => (
              <li
                key={track.id}
                className="flex items-center gap-3 p-2 rounded hover:bg-voice-surface/50 transition-colors"
              >
                {track.cover_art_url ? (
                  <img
                    src={track.cover_art_url}
                    alt={track.title}
                    className="w-10 h-10 rounded object-cover"
                  />
                ) : (
                  <div className="w-10 h-10 rounded bg-voice-border flex items-center justify-center">
                    <MusicIcon className="w-5 h-5 text-gray-500" />
                  </div>
                )}
                <div className="flex-1 min-w-0">
                  <p className="text-white text-sm truncate">{track.title}</p>
                  <p className="text-gray-400 text-xs truncate">{track.artist}</p>
                </div>
                <div className="text-xs text-gray-500">
                  {Math.round(track.match_score * 100)}%
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {tracks.length === 0 && !mood && (
        <div className="text-gray-400 text-sm">No music recommendations yet.</div>
      )}
    </div>
  );
}

function TranslationDisplay({
  result,
  streaming,
  isProcessing,
}: {
  result: TranslationResult | null;
  streaming: string;
  isProcessing: boolean;
}) {
  const { copied, copy } = useCopyToClipboard();
  const { isSpeaking, speak, stop } = useTextToSpeech();
  const displayText = isProcessing ? streaming : result?.translated;

  if (!displayText && !isProcessing) {
    return null;
  }

  const getLanguageName = (code: string): string => {
    const languages: Record<string, string> = {
      auto: 'Auto-detect',
      en: 'English',
      de: 'German',
      es: 'Spanish',
      fr: 'French',
      it: 'Italian',
      pt: 'Portuguese',
      nl: 'Dutch',
      ru: 'Russian',
      ja: 'Japanese',
      zh: 'Chinese',
      ko: 'Korean',
      ar: 'Arabic',
    };
    return languages[code] || code;
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
          Translation
          {result && (
            <span className="ml-2 normal-case">
              ({getLanguageName(result.source_language)} → {getLanguageName(result.target_language)})
            </span>
          )}
        </h3>
        {!isProcessing && displayText && (
          <div className="flex items-center gap-1">
            <SpeakButton isSpeaking={isSpeaking} onSpeak={() => speak(displayText)} onStop={stop} />
            <CopyButton copied={copied} onClick={() => copy(displayText)} />
          </div>
        )}
      </div>
      <div className="text-white text-sm leading-relaxed">
        {displayText}
        {isProcessing && <span className="animate-pulse">|</span>}
      </div>
    </div>
  );
}

function LoadingSpinner() {
  return (
    <svg
      className="w-5 h-5 animate-spin text-voice-primary"
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

function MusicIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="currentColor" viewBox="0 0 24 24">
      <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
    </svg>
  );
}

function CopyButton({ copied, onClick }: { copied: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="p-1 text-gray-500 hover:text-white transition-colors rounded"
      title="Copy to clipboard"
    >
      {copied ? <CheckIcon className="w-4 h-4 text-green-500" /> : <CopyIcon className="w-4 h-4" />}
    </button>
  );
}

function CopyIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
    </svg>
  );
}

function CheckIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
    </svg>
  );
}

function SpeakButton({
  isSpeaking,
  onSpeak,
  onStop,
}: {
  isSpeaking: boolean;
  onSpeak: () => void;
  onStop: () => void;
}) {
  return (
    <button
      onClick={isSpeaking ? onStop : onSpeak}
      className="p-1 text-gray-500 hover:text-white transition-colors rounded"
      title={isSpeaking ? 'Stop speaking' : 'Read aloud'}
    >
      {isSpeaking ? (
        <StopCircleIcon className="w-4 h-4 text-voice-primary" />
      ) : (
        <SpeakerIcon className="w-4 h-4" />
      )}
    </button>
  );
}

function SpeakerIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z"
      />
    </svg>
  );
}

function StopCircleIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z"
      />
    </svg>
  );
}
