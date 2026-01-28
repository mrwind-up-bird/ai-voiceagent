'use client';

import { useState, useCallback, useEffect, useRef } from 'react';
import { useVoiceStore, ActionItem, Track, TranslationResult, DevLogResult, BrainDumpResult, BrainDumpTask, EisenhowerQuadrant, MentalMirrorResult } from '../store/voiceStore';

const LANGUAGES = [
  { code: 'en', name: 'English' },
  { code: 'de', name: 'German' },
  { code: 'es', name: 'Spanish' },
  { code: 'fr', name: 'French' },
  { code: 'it', name: 'Italian' },
  { code: 'pt', name: 'Portuguese' },
  { code: 'nl', name: 'Dutch' },
  { code: 'ru', name: 'Russian' },
  { code: 'ja', name: 'Japanese' },
  { code: 'zh', name: 'Chinese' },
  { code: 'ko', name: 'Korean' },
  { code: 'ar', name: 'Arabic' },
];

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

function useOutputTranslation(originalText: string | null) {
  const [targetLanguage, setTargetLanguage] = useState<string | null>(null);
  const [translatedText, setTranslatedText] = useState<string | null>(null);
  const [isTranslating, setIsTranslating] = useState(false);
  const previousOriginal = useRef<string | null>(null);
  const previousLanguage = useRef<string | null>(null);

  // Reset translation when original changes
  useEffect(() => {
    if (originalText !== previousOriginal.current) {
      previousOriginal.current = originalText;
      setTranslatedText(null);
      // Re-translate if language was selected
      if (targetLanguage && originalText) {
        translateText(originalText, targetLanguage);
      }
    }
  }, [originalText, targetLanguage]);

  // Auto-translate when language changes
  useEffect(() => {
    if (targetLanguage !== previousLanguage.current && targetLanguage && originalText) {
      previousLanguage.current = targetLanguage;
      translateText(originalText, targetLanguage);
    }
  }, [targetLanguage, originalText]);

  const translateText = async (text: string, lang: string) => {
    if (!text || !lang) return;

    setIsTranslating(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const openaiKey = await invoke<string | null>('get_api_key', { keyType: 'openai' });

      if (!openaiKey) {
        console.error('OpenAI API key required for translation');
        setIsTranslating(false);
        return;
      }

      const result = await invoke<{ translated: string }>('translate_text', {
        apiKey: openaiKey,
        text,
        sourceLanguage: 'auto',
        targetLanguage: lang,
      });

      setTranslatedText(result.translated);
    } catch (err) {
      console.error('Translation error:', err);
    } finally {
      setIsTranslating(false);
    }
  };

  const clearTranslation = () => {
    setTargetLanguage(null);
    setTranslatedText(null);
    previousLanguage.current = null;
  };

  return {
    targetLanguage,
    setTargetLanguage,
    translatedText,
    isTranslating,
    clearTranslation,
  };
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
    devLogResult,
    devLogStreaming,
    brainDumpResult,
    brainDumpStreaming,
    mentalMirrorResult,
    mentalMirrorStreaming,
    isProcessing,
    processingMessage,
  } = useVoiceStore();

  if (!activeAgent) {
    return null;
  }

  return (
    <div className="w-full">
      <div className="glass rounded-lg p-4">
        {isProcessing && activeAgent !== 'tone-shifter' && activeAgent !== 'translator' && activeAgent !== 'action-items' && activeAgent !== 'dev-log' && activeAgent !== 'brain-dump' && activeAgent !== 'mental-mirror' && (
          <div className="flex items-center gap-3 text-gray-400">
            <LoadingSpinner />
            <span className="text-sm">{processingMessage || 'Processing...'}</span>
          </div>
        )}

        {activeAgent === 'action-items' && (
          <ActionItemsDisplay items={actionItems} isProcessing={isProcessing} />
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

        {activeAgent === 'dev-log' && (
          <DevLogDisplay
            result={devLogResult}
            streaming={devLogStreaming}
            isProcessing={isProcessing}
          />
        )}

        {activeAgent === 'brain-dump' && (
          <BrainDumpDisplay
            result={brainDumpResult}
            streaming={brainDumpStreaming}
            isProcessing={isProcessing}
          />
        )}

        {activeAgent === 'mental-mirror' && (
          <MentalMirrorDisplay
            result={mentalMirrorResult}
            streaming={mentalMirrorStreaming}
            isProcessing={isProcessing}
          />
        )}
      </div>
    </div>
  );
}

function ActionItemsDisplay({ items, isProcessing }: { items: ActionItem[]; isProcessing: boolean }) {
  const { copied, copy } = useCopyToClipboard();
  const { isSpeaking, speak, stop } = useTextToSpeech();
  const [showLanguageMenu, setShowLanguageMenu] = useState(false);

  const originalText = items.length > 0
    ? items.map((item, i) => {
        let line = `${i + 1}. [${item.priority.toUpperCase()}] ${item.task}`;
        if (item.assignee) line += ` (@${item.assignee})`;
        if (item.due_date) line += ` - Due: ${item.due_date}`;
        return line;
      }).join('\n')
    : null;

  const {
    targetLanguage,
    setTargetLanguage,
    translatedText,
    isTranslating,
    clearTranslation,
  } = useOutputTranslation(originalText);

  const displayText = translatedText || originalText;
  const showBlur = isProcessing || isTranslating;

  if (items.length === 0 && !isProcessing) {
    return (
      <div className="text-gray-400 text-sm">No action items found.</div>
    );
  }

  if (isProcessing && items.length === 0) {
    return (
      <div className="flex items-center gap-3 text-gray-400">
        <LoadingSpinner />
        <span className="text-sm">Extracting action items...</span>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
          Action Items ({items.length})
          {targetLanguage && (
            <span className="ml-2 normal-case text-voice-primary">
              → {LANGUAGES.find(l => l.code === targetLanguage)?.name}
            </span>
          )}
        </h3>
        <div className="flex items-center gap-1">
          <SpeakButton
            isSpeaking={isSpeaking}
            onSpeak={() => speak(displayText || '')}
            onStop={stop}
          />
          <div className="relative">
            <button
              onClick={() => setShowLanguageMenu(!showLanguageMenu)}
              className={`p-1 transition-colors rounded ${
                targetLanguage ? 'text-voice-primary' : 'text-gray-500 hover:text-white'
              }`}
              title="Translate"
            >
              <TranslateIcon className="w-4 h-4" />
            </button>
            {showLanguageMenu && (
              <LanguageMenu
                currentLanguage={targetLanguage}
                onSelect={(lang) => {
                  if (lang === targetLanguage) {
                    clearTranslation();
                  } else {
                    setTargetLanguage(lang);
                  }
                  setShowLanguageMenu(false);
                }}
                onClose={() => setShowLanguageMenu(false)}
              />
            )}
          </div>
          <CopyButton copied={copied} onClick={() => copy(displayText || '')} />
        </div>
      </div>
      <div className={`transition-all duration-300 ${showBlur ? 'blur-[2px] select-none' : 'blur-0'}`}>
        {translatedText ? (
          <div className="p-3 bg-voice-surface/30 rounded-lg border border-voice-border/50">
            <div className="text-white text-sm whitespace-pre-line leading-relaxed prose-invert">
              {translatedText.split('\n').map((line, i) => {
                const isNumbered = /^\d+\./.test(line.trim());
                const isBulleted = line.trim().startsWith('-') || line.trim().startsWith('•');
                const isHeader = line.includes('[') && line.includes(']');
                return (
                  <p key={i} className={`${i > 0 ? 'mt-1' : ''} ${isNumbered || isBulleted ? 'pl-2' : ''} ${isHeader ? 'font-medium' : ''}`}>
                    {line || '\u00A0'}
                  </p>
                );
              })}
            </div>
          </div>
        ) : (
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
        )}
      </div>
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
  const [showLanguageMenu, setShowLanguageMenu] = useState(false);

  const originalText = isProcessing ? streaming : result?.shifted || null;

  const {
    targetLanguage,
    setTargetLanguage,
    translatedText,
    isTranslating,
    clearTranslation,
  } = useOutputTranslation(result?.shifted || null);

  const displayText = translatedText || originalText;
  const showBlur = isProcessing || isTranslating;

  if (!displayText && !isProcessing) {
    return null;
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
          Tone Shifted {result?.tone && `(${result.tone})`}
          {targetLanguage && (
            <span className="ml-2 normal-case text-voice-primary">
              → {LANGUAGES.find(l => l.code === targetLanguage)?.name}
            </span>
          )}
        </h3>
        {displayText && (
          <div className="flex items-center gap-1">
            <SpeakButton
              isSpeaking={isSpeaking}
              onSpeak={() => speak(displayText)}
              onStop={stop}
            />
            <div className="relative">
              <button
                onClick={() => setShowLanguageMenu(!showLanguageMenu)}
                className={`p-1 transition-colors rounded ${
                  targetLanguage ? 'text-voice-primary' : 'text-gray-500 hover:text-white'
                }`}
                title="Translate"
                disabled={isProcessing}
              >
                <TranslateIcon className="w-4 h-4" />
              </button>
              {showLanguageMenu && !isProcessing && (
                <LanguageMenu
                  currentLanguage={targetLanguage}
                  onSelect={(lang) => {
                    if (lang === targetLanguage) {
                      clearTranslation();
                    } else {
                      setTargetLanguage(lang);
                    }
                    setShowLanguageMenu(false);
                  }}
                  onClose={() => setShowLanguageMenu(false)}
                />
              )}
            </div>
            <CopyButton copied={copied} onClick={() => copy(displayText)} />
          </div>
        )}
      </div>
      <div
        className={`p-3 bg-voice-surface/30 rounded-lg border border-voice-border/50 transition-all duration-300 ${
          showBlur ? 'blur-[2px] select-none' : 'blur-0'
        }`}
      >
        <p className="text-white text-sm leading-relaxed whitespace-pre-wrap">
          {displayText}
          {(isProcessing || isTranslating) && <span className="animate-pulse">|</span>}
        </p>
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
    return LANGUAGES.find(l => l.code === code)?.name || code;
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
      <div
        className={`p-3 bg-voice-surface/30 rounded-lg border border-voice-border/50 transition-all duration-300 ${
          isProcessing ? 'blur-[2px] select-none' : 'blur-0'
        }`}
      >
        <p className="text-white text-sm leading-relaxed whitespace-pre-wrap">
          {displayText}
          {isProcessing && <span className="animate-pulse">|</span>}
        </p>
      </div>
    </div>
  );
}

function DevLogDisplay({
  result,
  streaming,
  isProcessing,
}: {
  result: DevLogResult | null;
  streaming: string;
  isProcessing: boolean;
}) {
  const { copied: copiedCommit, copy: copyCommit } = useCopyToClipboard();
  const { copied: copiedTicket, copy: copyTicket } = useCopyToClipboard();
  const { copied: copiedSlack, copy: copySlack } = useCopyToClipboard();
  const { isSpeaking, speak, stop } = useTextToSpeech();
  const [expandedSection, setExpandedSection] = useState<string | null>(null);

  const showBlur = isProcessing;

  if (!result && !isProcessing) {
    return null;
  }

  if (isProcessing && !result) {
    return (
      <div className="flex items-center gap-3 text-gray-400">
        <LoadingSpinner />
        <span className="text-sm">Generating dev documentation...</span>
      </div>
    );
  }

  const toggleSection = (section: string) => {
    setExpandedSection(expandedSection === section ? null : section);
  };

  const formatTicketForCopy = () => {
    if (!result) return '';
    return `# ${result.ticket.title}\n\n## Description\n${result.ticket.description}\n\n## Acceptance Criteria\n${result.ticket.acceptance_criteria.map(ac => `- [ ] ${ac}`).join('\n')}`;
  };

  const speakAll = () => {
    if (!result) return;
    const fullText = `Commit message: ${result.commit_message}. Ticket: ${result.ticket.title}. ${result.ticket.description}. Slack update: ${result.slack_update}`;
    speak(fullText);
  };

  return (
    <div className={`transition-all duration-300 ${showBlur ? 'blur-[2px] select-none' : 'blur-0'}`}>
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
          Dev Documentation
        </h3>
        <div className="flex items-center gap-1">
          <SpeakButton isSpeaking={isSpeaking} onSpeak={speakAll} onStop={stop} />
        </div>
      </div>

      {result && (
        <div className="space-y-4">
          {/* Commit Message */}
          <div className="border border-voice-border rounded-lg overflow-hidden">
            <button
              onClick={() => toggleSection('commit')}
              className="w-full flex items-center justify-between px-3 py-2 bg-voice-surface/50 hover:bg-voice-surface transition-colors"
            >
              <div className="flex items-center gap-2">
                <GitCommitIcon className="w-4 h-4 text-green-500" />
                <span className="text-sm font-medium text-white">Commit Message</span>
              </div>
              <ChevronIcon className={`w-4 h-4 text-gray-400 transition-transform ${expandedSection === 'commit' ? 'rotate-180' : ''}`} />
            </button>
            {expandedSection === 'commit' && (
              <div className="p-3 border-t border-voice-border">
                <div className="flex items-start justify-between gap-2">
                  <pre className="text-sm text-white whitespace-pre-wrap font-mono flex-1">{result.commit_message}</pre>
                  <CopyButton copied={copiedCommit} onClick={() => copyCommit(result.commit_message)} />
                </div>
              </div>
            )}
          </div>

          {/* Jira/Linear Ticket */}
          <div className="border border-voice-border rounded-lg overflow-hidden">
            <button
              onClick={() => toggleSection('ticket')}
              className="w-full flex items-center justify-between px-3 py-2 bg-voice-surface/50 hover:bg-voice-surface transition-colors"
            >
              <div className="flex items-center gap-2">
                <TicketIcon className="w-4 h-4 text-blue-500" />
                <span className="text-sm font-medium text-white">Jira/Linear Ticket</span>
              </div>
              <ChevronIcon className={`w-4 h-4 text-gray-400 transition-transform ${expandedSection === 'ticket' ? 'rotate-180' : ''}`} />
            </button>
            {expandedSection === 'ticket' && (
              <div className="p-3 border-t border-voice-border">
                <div className="flex items-start justify-between gap-2 mb-3">
                  <h4 className="text-sm font-semibold text-white">{result.ticket.title}</h4>
                  <CopyButton copied={copiedTicket} onClick={() => copyTicket(formatTicketForCopy())} />
                </div>
                <p className="text-sm text-gray-300 mb-3">{result.ticket.description}</p>
                <div>
                  <span className="text-xs font-medium text-gray-400 uppercase tracking-wider">Acceptance Criteria</span>
                  <ul className="mt-2 space-y-1">
                    {result.ticket.acceptance_criteria.map((ac, i) => (
                      <li key={i} className="flex items-start gap-2 text-sm text-gray-300">
                        <span className="text-green-500 mt-0.5">•</span>
                        <span>{ac}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            )}
          </div>

          {/* Slack Update */}
          <div className="border border-voice-border rounded-lg overflow-hidden">
            <button
              onClick={() => toggleSection('slack')}
              className="w-full flex items-center justify-between px-3 py-2 bg-voice-surface/50 hover:bg-voice-surface transition-colors"
            >
              <div className="flex items-center gap-2">
                <SlackIcon className="w-4 h-4 text-purple-500" />
                <span className="text-sm font-medium text-white">Slack Update</span>
              </div>
              <ChevronIcon className={`w-4 h-4 text-gray-400 transition-transform ${expandedSection === 'slack' ? 'rotate-180' : ''}`} />
            </button>
            {expandedSection === 'slack' && (
              <div className="p-3 border-t border-voice-border">
                <div className="flex items-start justify-between gap-2">
                  <p className="text-sm text-white flex-1">{result.slack_update}</p>
                  <CopyButton copied={copiedSlack} onClick={() => copySlack(result.slack_update)} />
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

const QUADRANT_CONFIG: Record<EisenhowerQuadrant, { label: string; color: string; bgColor: string }> = {
  urgent_important: { label: 'Do First', color: 'text-red-400', bgColor: 'bg-red-500/20' },
  not_urgent_important: { label: 'Schedule', color: 'text-blue-400', bgColor: 'bg-blue-500/20' },
  urgent_not_important: { label: 'Delegate', color: 'text-amber-400', bgColor: 'bg-amber-500/20' },
  not_urgent_not_important: { label: 'Later', color: 'text-gray-400', bgColor: 'bg-gray-500/20' },
};

function BrainDumpDisplay({
  result,
  streaming,
  isProcessing,
}: {
  result: BrainDumpResult | null;
  streaming: string;
  isProcessing: boolean;
}) {
  const { copied, copy } = useCopyToClipboard();
  const { isSpeaking, speak, stop } = useTextToSpeech();
  const [showLanguageMenu, setShowLanguageMenu] = useState(false);
  const [activeTab, setActiveTab] = useState<'tasks' | 'ideas' | 'notes'>('tasks');

  const formatForText = (): string => {
    if (!result) return '';

    let text = `Summary: ${result.summary}\n\n`;

    if (result.tasks.length > 0) {
      text += 'TASKS:\n';
      result.tasks.forEach((task, i) => {
        const quadrant = QUADRANT_CONFIG[task.quadrant];
        text += `${i + 1}. [${quadrant.label}] ${task.title}: ${task.description}`;
        if (task.due_hint) text += ` (${task.due_hint})`;
        text += '\n';
      });
      text += '\n';
    }

    if (result.creative_ideas.length > 0) {
      text += 'IDEAS:\n';
      result.creative_ideas.forEach((idea, i) => {
        text += `${i + 1}. ${idea.title}: ${idea.description}\n`;
      });
      text += '\n';
    }

    if (result.notes.length > 0) {
      text += 'NOTES:\n';
      result.notes.forEach((note, i) => {
        text += `${i + 1}. ${note.content}\n`;
      });
    }

    return text;
  };

  const originalText = result ? formatForText() : null;

  const {
    targetLanguage,
    setTargetLanguage,
    translatedText,
    isTranslating,
    clearTranslation,
  } = useOutputTranslation(originalText);

  const showBlur = isProcessing || isTranslating;

  if (!result && !isProcessing) {
    return null;
  }

  if (isProcessing && !result) {
    return (
      <div className="flex items-center gap-3 text-gray-400">
        <LoadingSpinner />
        <span className="text-sm">Processing brain dump...</span>
      </div>
    );
  }

  const tasksByQuadrant: Partial<Record<EisenhowerQuadrant, BrainDumpTask[]>> = result?.tasks.reduce((acc, task) => {
    if (!acc[task.quadrant]) acc[task.quadrant] = [];
    acc[task.quadrant]!.push(task);
    return acc;
  }, {} as Partial<Record<EisenhowerQuadrant, BrainDumpTask[]>>) || {};

  const speakAll = () => {
    const text = translatedText || formatForText();
    speak(text);
  };

  return (
    <div className={`transition-all duration-300 ${showBlur ? 'blur-[2px] select-none' : 'blur-0'}`}>
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
          Brain Dump
          {targetLanguage && (
            <span className="ml-2 normal-case text-voice-primary">
              → {LANGUAGES.find(l => l.code === targetLanguage)?.name}
            </span>
          )}
        </h3>
        <div className="flex items-center gap-1">
          <SpeakButton isSpeaking={isSpeaking} onSpeak={speakAll} onStop={stop} />
          <div className="relative">
            <button
              onClick={() => setShowLanguageMenu(!showLanguageMenu)}
              className={`p-1 transition-colors rounded ${
                targetLanguage ? 'text-voice-primary' : 'text-gray-500 hover:text-white'
              }`}
              title="Translate"
              disabled={isProcessing}
            >
              <TranslateIcon className="w-4 h-4" />
            </button>
            {showLanguageMenu && !isProcessing && (
              <LanguageMenu
                currentLanguage={targetLanguage}
                onSelect={(lang) => {
                  if (lang === targetLanguage) {
                    clearTranslation();
                  } else {
                    setTargetLanguage(lang);
                  }
                  setShowLanguageMenu(false);
                }}
                onClose={() => setShowLanguageMenu(false)}
              />
            )}
          </div>
          <CopyButton copied={copied} onClick={() => copy(translatedText || formatForText())} />
        </div>
      </div>

      {/* Summary */}
      {result?.summary && (
        <div className="mb-4 p-3 bg-voice-surface/50 rounded-lg">
          <p className="text-sm text-gray-300 italic">{result.summary}</p>
        </div>
      )}

      {/* Show translated text if available */}
      {translatedText ? (
        <div className="p-3 bg-voice-surface/30 rounded-lg border border-voice-border/50">
          <div className="text-white text-sm leading-relaxed">
            {translatedText.split('\n').map((line, i) => {
              const trimmed = line.trim();
              const isHeader = trimmed === 'TASKS:' || trimmed === 'IDEAS:' || trimmed === 'NOTES:' || trimmed.startsWith('Summary:');
              const isNumbered = /^\d+\./.test(trimmed);
              const isEmpty = trimmed === '';
              return (
                <p
                  key={i}
                  className={`
                    ${isEmpty ? 'h-2' : ''}
                    ${isHeader ? 'font-semibold text-voice-primary mt-3 first:mt-0 uppercase text-xs tracking-wider' : ''}
                    ${isNumbered ? 'pl-2 mt-1' : ''}
                    ${!isHeader && !isNumbered && !isEmpty ? 'mt-1' : ''}
                  `}
                >
                  {line || '\u00A0'}
                </p>
              );
            })}
          </div>
        </div>
      ) : result && (
        <>
          {/* Tabs */}
          <div className="flex gap-1 mb-4 border-b border-voice-border">
            <button
              onClick={() => setActiveTab('tasks')}
              className={`px-3 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
                activeTab === 'tasks'
                  ? 'border-voice-primary text-voice-primary'
                  : 'border-transparent text-gray-400 hover:text-white'
              }`}
            >
              Tasks ({result.tasks.length})
            </button>
            <button
              onClick={() => setActiveTab('ideas')}
              className={`px-3 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
                activeTab === 'ideas'
                  ? 'border-voice-primary text-voice-primary'
                  : 'border-transparent text-gray-400 hover:text-white'
              }`}
            >
              Ideas ({result.creative_ideas.length})
            </button>
            <button
              onClick={() => setActiveTab('notes')}
              className={`px-3 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
                activeTab === 'notes'
                  ? 'border-voice-primary text-voice-primary'
                  : 'border-transparent text-gray-400 hover:text-white'
              }`}
            >
              Notes ({result.notes.length})
            </button>
          </div>

          {/* Tasks Tab - Eisenhower Matrix */}
          {activeTab === 'tasks' && (
            <div className="space-y-3">
              {result.tasks.length === 0 ? (
                <p className="text-gray-400 text-sm">No tasks identified.</p>
              ) : (
                <>
                  {/* Group by quadrant */}
                  {(['urgent_important', 'not_urgent_important', 'urgent_not_important', 'not_urgent_not_important'] as EisenhowerQuadrant[]).map((quadrant) => {
                    const tasks = tasksByQuadrant[quadrant] || [];
                    if (tasks.length === 0) return null;
                    const config = QUADRANT_CONFIG[quadrant];
                    return (
                      <div key={quadrant} className="space-y-2">
                        <div className={`inline-flex items-center gap-2 px-2 py-1 rounded ${config.bgColor}`}>
                          <span className={`text-xs font-medium ${config.color}`}>{config.label}</span>
                          <span className="text-xs text-gray-500">({tasks.length})</span>
                        </div>
                        <ul className="space-y-2 ml-2">
                          {tasks.map((task, i) => (
                            <li key={i} className="flex gap-3 p-2 rounded bg-voice-surface/30">
                              <div className="flex-1 min-w-0">
                                <p className="text-white text-sm font-medium">{task.title}</p>
                                <p className="text-gray-400 text-xs mt-1">{task.description}</p>
                                {task.due_hint && (
                                  <span className="inline-block mt-1 text-xs text-amber-400">
                                    {task.due_hint}
                                  </span>
                                )}
                              </div>
                            </li>
                          ))}
                        </ul>
                      </div>
                    );
                  })}
                </>
              )}
            </div>
          )}

          {/* Ideas Tab */}
          {activeTab === 'ideas' && (
            <div className="space-y-3">
              {result.creative_ideas.length === 0 ? (
                <p className="text-gray-400 text-sm">No creative ideas identified.</p>
              ) : (
                result.creative_ideas.map((idea, i) => (
                  <div key={i} className="p-3 rounded-lg bg-voice-surface/30 border border-voice-border">
                    <div className="flex items-start gap-2">
                      <LightbulbIcon className="w-4 h-4 text-yellow-400 mt-0.5 flex-shrink-0" />
                      <div className="flex-1 min-w-0">
                        <p className="text-white text-sm font-medium">{idea.title}</p>
                        <p className="text-gray-400 text-xs mt-1">{idea.description}</p>
                        <div className="flex flex-wrap gap-2 mt-2">
                          {idea.category && (
                            <span className="px-2 py-0.5 bg-purple-500/20 text-purple-400 rounded text-xs">
                              {idea.category}
                            </span>
                          )}
                          {idea.potential && (
                            <span className="text-xs text-gray-500 italic">{idea.potential}</span>
                          )}
                        </div>
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>
          )}

          {/* Notes Tab */}
          {activeTab === 'notes' && (
            <div className="space-y-3">
              {result.notes.length === 0 ? (
                <p className="text-gray-400 text-sm">No notes identified.</p>
              ) : (
                result.notes.map((note, i) => (
                  <div key={i} className="p-3 rounded-lg bg-voice-surface/30">
                    <p className="text-white text-sm">{note.content}</p>
                    {note.tags.length > 0 && (
                      <div className="flex flex-wrap gap-1 mt-2">
                        {note.tags.map((tag, j) => (
                          <span key={j} className="text-xs text-gray-500">#{tag}</span>
                        ))}
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}

function MentalMirrorDisplay({
  result,
  streaming,
  isProcessing,
}: {
  result: MentalMirrorResult | null;
  streaming: string;
  isProcessing: boolean;
}) {
  const { copied, copy } = useCopyToClipboard();
  const { isSpeaking, speak, stop } = useTextToSpeech();
  const [showLanguageMenu, setShowLanguageMenu] = useState(false);

  const formatForText = (): string => {
    if (!result) return '';
    return `LETTER TO MY FUTURE SELF
${result.date}

REFLECTION
${result.reflection}

MENTAL CHECK-IN
${result.mental_checkin}

THE RELEASE
${result.the_release}

MESSAGE TO TOMORROW
${result.message_to_tomorrow}

---
${result.disclaimer}`;
  };

  const originalText = result ? formatForText() : null;

  const {
    targetLanguage,
    setTargetLanguage,
    translatedText,
    isTranslating,
    clearTranslation,
  } = useOutputTranslation(originalText);

  const showBlur = isProcessing || isTranslating;

  if (!result && !isProcessing) {
    return null;
  }

  if (isProcessing && !result) {
    return (
      <div className="flex flex-col items-center justify-center py-8 text-gray-400">
        <div className="relative mb-4">
          <LetterHeartIcon className="w-12 h-12 text-pink-400/50 animate-pulse" />
        </div>
        <span className="text-sm">Creating your letter with care...</span>
      </div>
    );
  }

  const speakLetter = () => {
    const text = translatedText || formatForText();
    speak(text);
  };

  return (
    <div className={`transition-all duration-500 ${showBlur ? 'blur-[3px] select-none' : 'blur-0'}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <LetterHeartIcon className="w-5 h-5 text-pink-400" />
          <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
            Letter to My Future Self
            {targetLanguage && (
              <span className="ml-2 normal-case text-voice-primary">
                → {LANGUAGES.find(l => l.code === targetLanguage)?.name}
              </span>
            )}
          </h3>
        </div>
        <div className="flex items-center gap-1">
          <SpeakButton isSpeaking={isSpeaking} onSpeak={speakLetter} onStop={stop} />
          <div className="relative">
            <button
              onClick={() => setShowLanguageMenu(!showLanguageMenu)}
              className={`p-1 transition-colors rounded ${
                targetLanguage ? 'text-voice-primary' : 'text-gray-500 hover:text-white'
              }`}
              title="Translate"
              disabled={isProcessing}
            >
              <TranslateIcon className="w-4 h-4" />
            </button>
            {showLanguageMenu && !isProcessing && (
              <LanguageMenu
                currentLanguage={targetLanguage}
                onSelect={(lang) => {
                  if (lang === targetLanguage) {
                    clearTranslation();
                  } else {
                    setTargetLanguage(lang);
                  }
                  setShowLanguageMenu(false);
                }}
                onClose={() => setShowLanguageMenu(false)}
              />
            )}
          </div>
          <CopyButton copied={copied} onClick={() => copy(translatedText || formatForText())} />
        </div>
      </div>

      {/* Show translated text if available */}
      {translatedText ? (
        <div className="p-4 bg-gradient-to-br from-pink-950/20 via-voice-surface/30 to-purple-950/20 rounded-xl border border-pink-500/20">
          <div className="text-white text-sm leading-relaxed">
            {translatedText.split('\n').map((line, i) => {
              const trimmed = line.trim();
              const isHeader = ['LETTER TO MY FUTURE SELF', 'REFLECTION', 'MENTAL CHECK-IN', 'THE RELEASE', 'MESSAGE TO TOMORROW'].includes(trimmed);
              const isDate = trimmed.includes(',') && (trimmed.includes('January') || trimmed.includes('February') || trimmed.includes('March') || trimmed.includes('April') || trimmed.includes('May') || trimmed.includes('June') || trimmed.includes('July') || trimmed.includes('August') || trimmed.includes('September') || trimmed.includes('October') || trimmed.includes('November') || trimmed.includes('December'));
              const isDivider = trimmed === '---';
              const isEmpty = trimmed === '';
              return (
                <p
                  key={i}
                  className={`
                    ${isEmpty ? 'h-3' : ''}
                    ${isHeader ? 'font-semibold text-pink-300 mt-4 first:mt-0 text-sm tracking-wide' : ''}
                    ${isDate ? 'text-gray-400 text-xs italic mb-2' : ''}
                    ${isDivider ? 'border-t border-pink-500/20 my-4' : ''}
                    ${!isHeader && !isDate && !isDivider && !isEmpty ? 'mt-1' : ''}
                  `}
                >
                  {isDivider ? null : line || '\u00A0'}
                </p>
              );
            })}
          </div>
        </div>
      ) : result && (
        <div className="space-y-0">
          {/* Letter Card */}
          <div className="relative bg-gradient-to-br from-pink-950/20 via-voice-surface/30 to-purple-950/20 rounded-xl border border-pink-500/20 overflow-hidden">
            {/* Decorative corner */}
            <div className="absolute top-0 right-0 w-16 h-16 bg-gradient-to-bl from-pink-500/10 to-transparent" />

            {/* Date */}
            <div className="px-5 pt-4 pb-2">
              <p className="text-xs text-gray-400 italic">{result.date}</p>
            </div>

            {/* Letter Sections */}
            <div className="px-5 pb-5 space-y-5">
              {/* Reflection */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <div className="w-1 h-4 bg-pink-400 rounded-full" />
                  <h4 className="text-xs font-semibold text-pink-300 uppercase tracking-wider">Reflection</h4>
                </div>
                <p className="text-sm text-gray-200 leading-relaxed pl-3">{result.reflection}</p>
              </div>

              {/* Mental Check-in */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <div className="w-1 h-4 bg-blue-400 rounded-full" />
                  <h4 className="text-xs font-semibold text-blue-300 uppercase tracking-wider">Mental Check-in</h4>
                </div>
                <p className="text-sm text-gray-200 leading-relaxed pl-3">{result.mental_checkin}</p>
              </div>

              {/* The Release */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <div className="w-1 h-4 bg-purple-400 rounded-full" />
                  <h4 className="text-xs font-semibold text-purple-300 uppercase tracking-wider">The Release</h4>
                </div>
                <p className="text-sm text-gray-200 leading-relaxed pl-3">{result.the_release}</p>
              </div>

              {/* Message to Tomorrow */}
              <div className="pt-2 border-t border-pink-500/20">
                <div className="flex items-center gap-2 mb-2">
                  <div className="w-1 h-4 bg-amber-400 rounded-full" />
                  <h4 className="text-xs font-semibold text-amber-300 uppercase tracking-wider">Message to Tomorrow</h4>
                </div>
                <p className="text-sm text-gray-200 leading-relaxed pl-3 italic">{result.message_to_tomorrow}</p>
              </div>
            </div>

            {/* Disclaimer */}
            <div className="px-5 py-3 bg-black/20 border-t border-pink-500/10">
              <p className="text-[10px] text-gray-500 leading-relaxed">
                <ShieldIcon className="w-3 h-3 inline mr-1 opacity-50" />
                {result.disclaimer}
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function LanguageMenu({
  currentLanguage,
  onSelect,
  onClose,
}: {
  currentLanguage: string | null;
  onSelect: (lang: string) => void;
  onClose: () => void;
}) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  return (
    <div
      ref={menuRef}
      className="absolute right-0 top-full mt-1 z-50 bg-voice-surface border border-voice-border rounded-lg shadow-xl py-1 min-w-[140px] max-h-[200px] overflow-y-auto"
    >
      {LANGUAGES.map((lang) => (
        <button
          key={lang.code}
          onClick={() => onSelect(lang.code)}
          className={`w-full px-3 py-1.5 text-left text-sm transition-colors ${
            currentLanguage === lang.code
              ? 'bg-voice-primary/20 text-voice-primary'
              : 'text-gray-300 hover:bg-voice-border/50'
          }`}
        >
          {lang.name}
          {currentLanguage === lang.code && (
            <span className="ml-2 text-xs">(clear)</span>
          )}
        </button>
      ))}
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

function TranslateIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" />
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

function GitCommitIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <circle cx="12" cy="12" r="3" />
      <path strokeLinecap="round" strokeLinejoin="round" d="M12 3v6m0 6v6" />
    </svg>
  );
}

function TicketIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M15 5v2m0 4v2m0 4v2M5 5a2 2 0 00-2 2v3a2 2 0 110 4v3a2 2 0 002 2h14a2 2 0 002-2v-3a2 2 0 110-4V7a2 2 0 00-2-2H5z"
      />
    </svg>
  );
}

function SlackIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
      />
    </svg>
  );
}

function ChevronIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
    </svg>
  );
}

function LightbulbIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
      />
    </svg>
  );
}

function LetterHeartIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M21.75 9v.906a2.25 2.25 0 01-1.183 1.981l-6.478 3.488M2.25 9v.906a2.25 2.25 0 001.183 1.981l6.478 3.488m8.839 2.51l-4.66-2.51m0 0l-1.023-.55a2.25 2.25 0 00-2.134 0l-1.022.55m0 0l-4.661 2.51m16.5 1.615a2.25 2.25 0 01-2.25 2.25h-15a2.25 2.25 0 01-2.25-2.25V8.844a2.25 2.25 0 011.183-1.98l7.5-4.04a2.25 2.25 0 012.134 0l7.5 4.04a2.25 2.25 0 011.183 1.98V19.5z"
      />
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M12 10.5l-1.5-1.5a2.121 2.121 0 00-3 3l4.5 4.5 4.5-4.5a2.121 2.121 0 00-3-3L12 10.5z"
      />
    </svg>
  );
}

function ShieldIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
      />
    </svg>
  );
}
