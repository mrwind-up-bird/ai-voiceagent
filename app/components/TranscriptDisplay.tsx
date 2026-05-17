'use client';

import { useState, useCallback } from 'react';
import { useVoiceStore } from '../store/voiceStore';

// Sub-Project C — distinct hue per speaker (HSL: stable, accessible
// contrast against the dark glass background, no two adjacent
// speakers share a hue).
const SPEAKER_HUES = [200, 30, 280, 130, 0, 60, 320, 170];
function speakerColor(speaker: number): string {
  const hue = SPEAKER_HUES[speaker % SPEAKER_HUES.length];
  return `hsl(${hue}, 70%, 60%)`;
}
function speakerLabel(speaker: number | null | undefined): string {
  if (speaker === null || speaker === undefined) return '';
  return `Speaker ${speaker + 1}`;
}

export function TranscriptDisplay() {
  const { transcript, interimTranscript, recordingState, transcriptSegments } =
    useVoiceStore();
  const [copied, setCopied] = useState(false);
  const hasMultipleSpeakers =
    new Set(
      transcriptSegments
        .map((s) => s.speaker)
        .filter((s): s is number => s !== null)
    ).size > 1;

  const copyToClipboard = useCallback(async () => {
    if (!transcript) return;
    try {
      await navigator.clipboard.writeText(transcript);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }, [transcript]);

  const isRecording = recordingState === 'recording';
  const hasContent = transcript || interimTranscript;

  if (!hasContent && !isRecording) {
    return null;
  }

  return (
    <div className="w-full">
      <div className="glass rounded-lg p-4">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-gray-400 uppercase tracking-wider">
            Transcript
          </span>
          <div className="flex items-center gap-2">
            {isRecording && (
              <span className="flex items-center gap-1.5">
                <span className="w-2 h-2 bg-red-500 rounded-full recording-pulse" />
                <span className="text-xs text-red-400">Live</span>
              </span>
            )}
            {transcript && (
              <button
                onClick={copyToClipboard}
                className="p-1 text-gray-500 hover:text-white transition-colors rounded"
                title="Copy to clipboard"
              >
                {copied ? <CheckIcon className="w-4 h-4 text-green-500" /> : <CopyIcon className="w-4 h-4" />}
              </button>
            )}
          </div>
        </div>

        <div className="text-white text-sm leading-relaxed min-h-[60px] max-h-[200px] overflow-y-auto">
          {hasMultipleSpeakers ? (
            // Sub-Project C — render speaker-tagged segments when more
            // than one voice has been detected this session.
            <div className="flex flex-col gap-2">
              {transcriptSegments.map((seg, i) => (
                <div key={i} className="flex items-start gap-2">
                  {seg.speaker !== null && (
                    <span
                      className="shrink-0 text-[10px] font-medium px-1.5 py-0.5 rounded uppercase tracking-wider"
                      style={{
                        color: speakerColor(seg.speaker),
                        background: `${speakerColor(seg.speaker)}22`,
                        border: `1px solid ${speakerColor(seg.speaker)}55`,
                      }}
                    >
                      {speakerLabel(seg.speaker)}
                    </span>
                  )}
                  <span>{seg.text}</span>
                </div>
              ))}
              {interimTranscript && (
                <span className="text-gray-400 italic">{interimTranscript}</span>
              )}
            </div>
          ) : (
            <>
              {transcript && <span>{transcript}</span>}
              {interimTranscript && (
                <span className="text-gray-400 italic"> {interimTranscript}</span>
              )}
            </>
          )}
          {!transcript && !interimTranscript && isRecording && (
            <span className="text-gray-500">Waiting for speech...</span>
          )}
        </div>
      </div>
    </div>
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
