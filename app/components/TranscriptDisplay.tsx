'use client';

import { useState, useCallback } from 'react';
import { useVoiceStore } from '../store/voiceStore';

export function TranscriptDisplay() {
  const { transcript, interimTranscript, recordingState } = useVoiceStore();
  const [copied, setCopied] = useState(false);

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
          {transcript && <span>{transcript}</span>}
          {interimTranscript && (
            <span className="text-gray-400 italic"> {interimTranscript}</span>
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
