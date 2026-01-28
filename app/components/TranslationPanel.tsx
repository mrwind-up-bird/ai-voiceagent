'use client';

import { useVoiceStore } from '../store/voiceStore';

const LANGUAGES = [
  { code: 'auto', name: 'Auto-detect', sourceOnly: true },
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

export function TranslationPanel() {
  const {
    activeAgent,
    selectedSourceLanguage,
    selectedTargetLanguage,
    setSelectedSourceLanguage,
    setSelectedTargetLanguage,
  } = useVoiceStore();

  if (activeAgent !== 'translator') {
    return null;
  }

  const sourceLanguages = LANGUAGES;
  const targetLanguages = LANGUAGES.filter((lang) => !lang.sourceOnly);

  return (
    <div className="flex items-center gap-3 px-4 py-2">
      <div className="flex items-center gap-2">
        <label className="text-xs text-gray-400">From:</label>
        <select
          value={selectedSourceLanguage}
          onChange={(e) => setSelectedSourceLanguage(e.target.value)}
          className="bg-voice-surface text-white text-sm rounded px-2 py-1 border border-voice-border focus:outline-none focus:border-voice-primary"
        >
          {sourceLanguages.map((lang) => (
            <option key={lang.code} value={lang.code}>
              {lang.name}
            </option>
          ))}
        </select>
      </div>

      <SwapIcon
        className="w-4 h-4 text-gray-500 cursor-pointer hover:text-white transition-colors"
        onClick={() => {
          if (selectedSourceLanguage !== 'auto') {
            const temp = selectedSourceLanguage;
            setSelectedSourceLanguage(selectedTargetLanguage);
            setSelectedTargetLanguage(temp);
          }
        }}
      />

      <div className="flex items-center gap-2">
        <label className="text-xs text-gray-400">To:</label>
        <select
          value={selectedTargetLanguage}
          onChange={(e) => setSelectedTargetLanguage(e.target.value)}
          className="bg-voice-surface text-white text-sm rounded px-2 py-1 border border-voice-border focus:outline-none focus:border-voice-primary"
        >
          {targetLanguages.map((lang) => (
            <option key={lang.code} value={lang.code}>
              {lang.name}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

function SwapIcon({ className, onClick }: { className?: string; onClick?: () => void }) {
  return (
    <svg
      className={className}
      onClick={onClick}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4"
      />
    </svg>
  );
}
