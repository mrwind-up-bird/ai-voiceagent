'use client';

import { useState, useCallback } from 'react';
import { useVoiceStore } from '../store/voiceStore';

interface SyncPairingProps {
  createSession: () => Promise<string | null>;
  joinSession: (code: string) => Promise<boolean>;
  leaveSession: () => Promise<void>;
  onClose: () => void;
}

export function SyncPairing({ createSession, joinSession, leaveSession, onClose }: SyncPairingProps) {
  const { syncStatus, pairingCode, pairedDeviceName, syncWarning } = useVoiceStore();
  const [joinCode, setJoinCode] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCreate = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    const code = await createSession();
    if (!code) {
      setError('Failed to create session');
    }
    setIsLoading(false);
  }, [createSession]);

  const handleJoin = useCallback(async () => {
    const code = joinCode.trim().toLowerCase();
    if (!code) return;
    setIsLoading(true);
    setError(null);
    const ok = await joinSession(code);
    if (!ok) {
      setError('Failed to connect. Check the code and try again.');
    }
    setIsLoading(false);
  }, [joinCode, joinSession]);

  const handleLeave = useCallback(async () => {
    await leaveSession();
  }, [leaveSession]);

  const copyCode = useCallback(() => {
    if (pairingCode) {
      navigator.clipboard.writeText(pairingCode).catch(() => {});
    }
  }, [pairingCode]);

  // Connected state
  if (syncStatus === 'connected') {
    return (
      <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
        <div className="w-80 bg-voice-surface border border-white/10 rounded-xl p-6 shadow-2xl">
          <div className="flex items-center gap-3 mb-4">
            <span className="w-3 h-3 rounded-full bg-emerald-400 animate-pulse" />
            <h3 className="text-sm font-medium text-white">Synced</h3>
          </div>

          <p className="text-sm text-gray-300 mb-1">
            Connected to <span className="text-white font-medium">{pairedDeviceName || 'peer'}</span>
          </p>
          <p className="text-xs text-gray-500 mb-4">
            Transcript and agent results sync in real time.
          </p>

          {syncWarning && (
            <div className="mb-4 p-2 bg-amber-500/10 border border-amber-500/20 rounded-lg">
              <p className="text-xs text-amber-400">{syncWarning}</p>
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={handleLeave}
              className="flex-1 px-3 py-2 text-sm font-medium rounded-lg bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
            >
              Disconnect
            </button>
            <button
              onClick={onClose}
              className="flex-1 px-3 py-2 text-sm font-medium rounded-lg bg-white/5 text-gray-300 hover:bg-white/10 transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Waiting for peer
  if (syncStatus === 'waiting_for_peer' && pairingCode) {
    return (
      <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
        <div className="w-80 bg-voice-surface border border-white/10 rounded-xl p-6 shadow-2xl">
          <div className="flex items-center gap-3 mb-4">
            <span className="w-3 h-3 rounded-full bg-blue-400 animate-pulse" />
            <h3 className="text-sm font-medium text-white">Waiting for peer</h3>
          </div>

          <p className="text-xs text-gray-400 mb-3">
            Share this code with the other device:
          </p>

          <button
            onClick={copyCode}
            className="w-full flex items-center justify-center gap-2 px-4 py-3 mb-4 rounded-lg bg-white/5 border border-white/10 hover:bg-white/10 transition-colors group"
            title="Click to copy"
          >
            <span className="text-lg font-mono font-semibold tracking-widest text-white">
              {pairingCode}
            </span>
            <CopyIcon className="w-4 h-4 text-gray-500 group-hover:text-gray-300 transition-colors" />
          </button>

          <p className="text-xs text-gray-500 mb-4 text-center">
            Code expires in 60 seconds
          </p>

          <div className="flex gap-2">
            <button
              onClick={handleLeave}
              className="flex-1 px-3 py-2 text-sm font-medium rounded-lg bg-white/5 text-gray-300 hover:bg-white/10 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Connecting state
  if (syncStatus === 'connecting') {
    return (
      <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
        <div className="w-80 bg-voice-surface border border-white/10 rounded-xl p-6 shadow-2xl">
          <div className="flex items-center gap-3 mb-4">
            <LoadingSpinner className="w-4 h-4 text-amber-400" />
            <h3 className="text-sm font-medium text-white">Connecting...</h3>
          </div>
          <p className="text-xs text-gray-400">
            Establishing encrypted connection with peer.
          </p>
        </div>
      </div>
    );
  }

  // Disconnected — show create/join options
  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="w-80 bg-voice-surface border border-white/10 rounded-xl p-6 shadow-2xl">
        <div className="flex items-center justify-between mb-5">
          <h3 className="text-sm font-medium text-white">Sync Devices</h3>
          <button
            onClick={onClose}
            className="p-1 text-gray-500 hover:text-gray-300 transition-colors"
          >
            <CloseIcon className="w-4 h-4" />
          </button>
        </div>

        {error && (
          <div className="mb-4 p-2 bg-red-500/10 border border-red-500/20 rounded-lg">
            <p className="text-xs text-red-400">{error}</p>
          </div>
        )}

        {/* Create session */}
        <button
          onClick={handleCreate}
          disabled={isLoading}
          className="w-full flex items-center gap-3 px-4 py-3 mb-3 rounded-lg bg-voice-primary/10 border border-voice-primary/20 hover:bg-voice-primary/20 transition-colors disabled:opacity-50"
        >
          <DeviceIcon className="w-5 h-5 text-voice-primary" />
          <div className="text-left">
            <p className="text-sm font-medium text-white">Create Session</p>
            <p className="text-xs text-gray-400">Generate a pairing code</p>
          </div>
        </button>

        {/* Divider */}
        <div className="flex items-center gap-3 my-3">
          <div className="flex-1 h-px bg-white/10" />
          <span className="text-xs text-gray-500">or</span>
          <div className="flex-1 h-px bg-white/10" />
        </div>

        {/* Join session */}
        <div className="space-y-2">
          <input
            type="text"
            value={joinCode}
            onChange={(e) => setJoinCode(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleJoin()}
            placeholder="Enter pairing code"
            className="w-full px-4 py-2.5 text-sm rounded-lg bg-white/5 border border-white/10 text-white placeholder-gray-500 focus:outline-none focus:border-voice-primary/50 font-mono tracking-wide"
            disabled={isLoading}
          />
          <button
            onClick={handleJoin}
            disabled={isLoading || !joinCode.trim()}
            className="w-full px-4 py-2.5 text-sm font-medium rounded-lg bg-voice-primary text-white hover:bg-voice-secondary transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isLoading ? 'Connecting...' : 'Join Session'}
          </button>
        </div>

        <p className="mt-4 text-xs text-gray-500 text-center leading-relaxed">
          End-to-end encrypted. No data stored on servers.
        </p>
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

function CloseIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

function DeviceIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z" />
    </svg>
  );
}

function LoadingSpinner({ className }: { className?: string }) {
  return (
    <svg className={`${className} animate-spin`} fill="none" viewBox="0 0 24 24">
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
    </svg>
  );
}
