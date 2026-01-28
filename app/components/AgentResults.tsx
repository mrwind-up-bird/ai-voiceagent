'use client';

import { useVoiceStore, AgentType, ActionItem, Track } from '../store/voiceStore';

export function AgentResults() {
  const {
    activeAgent,
    actionItems,
    toneShiftResult,
    toneShiftStreaming,
    musicTracks,
    moodAnalysis,
    isProcessing,
    processingMessage,
  } = useVoiceStore();

  if (!activeAgent) {
    return null;
  }

  return (
    <div className="w-full max-w-xl">
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
      </div>
    </div>
  );
}

function ActionItemsDisplay({ items }: { items: ActionItem[] }) {
  if (items.length === 0) {
    return (
      <div className="text-gray-400 text-sm">No action items found.</div>
    );
  }

  return (
    <div>
      <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider mb-3">
        Action Items ({items.length})
      </h3>
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
  const displayText = isProcessing ? streaming : result?.shifted;

  if (!displayText && !isProcessing) {
    return null;
  }

  return (
    <div>
      <h3 className="text-xs font-medium text-gray-400 uppercase tracking-wider mb-3">
        Tone Shifted {result?.tone && `(${result.tone})`}
      </h3>
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
