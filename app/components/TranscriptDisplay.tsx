'use client';

import { useVoiceStore } from '../store/voiceStore';

export function TranscriptDisplay() {
  const { transcript, interimTranscript, recordingState } = useVoiceStore();

  const isRecording = recordingState === 'recording';
  const hasContent = transcript || interimTranscript;

  if (!hasContent && !isRecording) {
    return null;
  }

  return (
    <div className="w-full max-w-xl">
      <div className="glass rounded-lg p-4">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-gray-400 uppercase tracking-wider">
            Transcript
          </span>
          {isRecording && (
            <span className="flex items-center gap-1.5">
              <span className="w-2 h-2 bg-red-500 rounded-full recording-pulse" />
              <span className="text-xs text-red-400">Live</span>
            </span>
          )}
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
