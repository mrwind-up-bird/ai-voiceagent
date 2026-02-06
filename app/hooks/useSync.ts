'use client';

import { useEffect, useCallback } from 'react';
import { useVoiceStore, SyncStatus, PeerInfo } from '../store/voiceStore';

interface SyncStatusPayload {
  status: SyncStatus;
  session_id: string | null;
  pairing_code: string | null;
  peer: PeerInfo | null;
}

interface SyncSnapshotPayload {
  transcript: string;
  recording_state: string;
  active_agent: string | null;
  action_items: string | null;
  tone_shift: string | null;
  translation: string | null;
  dev_log: string | null;
  brain_dump: string | null;
  mental_mirror: string | null;
  music: string | null;
}

export function useSync() {
  const {
    setSyncStatus,
    setPairingCode,
    setPairedDeviceName,
    setSyncPeer,
    setTranscript,
    setRecordingState,
    setActiveAgent,
    setActionItems,
    setToneShiftResult,
    setTranslationResult,
    setDevLogResult,
    setBrainDumpResult,
    setMentalMirrorResult,
    setMusicTracks,
    setMoodAnalysis,
  } = useVoiceStore();

  // Listen for sync status changes from Rust backend
  useEffect(() => {
    let listeners: Array<() => void> = [];

    async function setupListeners() {
      try {
        const { listen } = await import('@tauri-apps/api/event');

        const unlistenStatus = await listen<SyncStatusPayload>(
          'sync-status-changed',
          (event) => {
            const { status, pairing_code, peer } = event.payload;
            setSyncStatus(status);
            setPairingCode(pairing_code);
            if (peer) {
              setPairedDeviceName(peer.device_name);
              setSyncPeer(peer);
            } else {
              setPairedDeviceName(null);
              setSyncPeer(null);
            }
          }
        );
        listeners.push(unlistenStatus);

        // Listen for remote state updates (snapshot from peer)
        const unlistenSnapshot = await listen<SyncSnapshotPayload>(
          'sync-state-updated',
          (event) => {
            const snap = event.payload;

            // Apply remote state to local store
            if (snap.transcript) {
              setTranscript(snap.transcript);
            }
            if (snap.recording_state) {
              setRecordingState(snap.recording_state as 'idle' | 'recording' | 'processing');
            }
            setActiveAgent(
              (snap.active_agent as ReturnType<typeof useVoiceStore.getState>['activeAgent']) ?? null
            );

            // Apply agent results (stored as JSON strings in yrs doc)
            if (snap.action_items) {
              try {
                const parsed = JSON.parse(snap.action_items);
                if (parsed.items) setActionItems(parsed.items);
              } catch { /* ignore parse errors */ }
            }
            if (snap.tone_shift) {
              try { setToneShiftResult(JSON.parse(snap.tone_shift)); } catch { /* */ }
            }
            if (snap.translation) {
              try { setTranslationResult(JSON.parse(snap.translation)); } catch { /* */ }
            }
            if (snap.dev_log) {
              try { setDevLogResult(JSON.parse(snap.dev_log)); } catch { /* */ }
            }
            if (snap.brain_dump) {
              try { setBrainDumpResult(JSON.parse(snap.brain_dump)); } catch { /* */ }
            }
            if (snap.mental_mirror) {
              try { setMentalMirrorResult(JSON.parse(snap.mental_mirror)); } catch { /* */ }
            }
            if (snap.music) {
              try {
                const parsed = JSON.parse(snap.music);
                if (parsed.tracks) setMusicTracks(parsed.tracks);
                if (parsed.analysis) setMoodAnalysis(parsed.analysis);
              } catch { /* */ }
            }
          }
        );
        listeners.push(unlistenSnapshot);
      } catch (error) {
        console.log('Sync events not available:', error);
      }
    }

    setupListeners();

    return () => {
      listeners.forEach((unlisten) => unlisten());
    };
  }, [
    setSyncStatus,
    setPairingCode,
    setPairedDeviceName,
    setSyncPeer,
    setTranscript,
    setRecordingState,
    setActiveAgent,
    setActionItems,
    setToneShiftResult,
    setTranslationResult,
    setDevLogResult,
    setBrainDumpResult,
    setMentalMirrorResult,
    setMusicTracks,
    setMoodAnalysis,
  ]);

  // Actions
  const createSession = useCallback(async (): Promise<string | null> => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const code = await invoke<string>('create_sync_session');
      return code;
    } catch (error) {
      console.error('Failed to create sync session:', error);
      return null;
    }
  }, []);

  const joinSession = useCallback(async (pairingCode: string): Promise<boolean> => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('join_sync_session', { pairingCode });
      return true;
    } catch (error) {
      console.error('Failed to join sync session:', error);
      return false;
    }
  }, []);

  const leaveSession = useCallback(async (): Promise<void> => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('leave_sync_session');
    } catch (error) {
      console.error('Failed to leave sync session:', error);
    }
  }, []);

  return { createSession, joinSession, leaveSession };
}
