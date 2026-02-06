'use client';

import { useVoiceStore } from '../store/voiceStore';

export default function SyncStatus() {
  const { syncStatus, pairedDeviceName, pairingCode } = useVoiceStore();

  if (syncStatus === 'disconnected') {
    return null;
  }

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium bg-white/5 border border-white/10">
      <span
        className={`w-2 h-2 rounded-full ${
          syncStatus === 'connected'
            ? 'bg-emerald-400 animate-pulse'
            : syncStatus === 'connecting'
            ? 'bg-amber-400 animate-pulse'
            : 'bg-blue-400 animate-pulse'
        }`}
      />
      <span className="text-white/70">
        {syncStatus === 'connected' && pairedDeviceName
          ? `Synced with ${pairedDeviceName}`
          : syncStatus === 'connecting'
          ? 'Connecting...'
          : syncStatus === 'waiting_for_peer' && pairingCode
          ? `Code: ${pairingCode}`
          : 'Sync'}
      </span>
    </div>
  );
}
