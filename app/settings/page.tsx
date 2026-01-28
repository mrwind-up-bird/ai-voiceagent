'use client';

import { useState, useEffect, useCallback } from 'react';
import Link from 'next/link';
import { WindowControls } from '../components/WindowControls';

interface ApiKeyConfig {
  id: string;
  name: string;
  description: string;
  placeholder: string;
}

const API_KEYS: ApiKeyConfig[] = [
  {
    id: 'deepgram',
    name: 'Deepgram',
    description: 'Real-time speech-to-text transcription',
    placeholder: 'Enter your Deepgram API key',
  },
  {
    id: 'assembly_ai',
    name: 'AssemblyAI',
    description: 'Fallback transcription service',
    placeholder: 'Enter your AssemblyAI API key',
  },
  {
    id: 'openai',
    name: 'OpenAI',
    description: 'GPT-4o for action item extraction',
    placeholder: 'Enter your OpenAI API key',
  },
  {
    id: 'anthropic',
    name: 'Anthropic',
    description: 'Claude for tone shifting',
    placeholder: 'Enter your Anthropic API key',
  },
  {
    id: 'qrecords',
    name: 'Q-Records',
    description: 'Music matching service',
    placeholder: 'Enter your Q-Records API key',
  },
];

export default function SettingsPage() {
  const [keyStates, setKeyStates] = useState<Record<string, { hasKey: boolean; isEditing: boolean; value: string }>>({});
  const [saving, setSaving] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Load key states on mount
  const loadKeyStates = useCallback(async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const states: Record<string, { hasKey: boolean; isEditing: boolean; value: string }> = {};

      for (const key of API_KEYS) {
        const existingKey = await invoke<string | null>('get_api_key', { keyType: key.id });
        states[key.id] = {
          hasKey: !!existingKey,
          isEditing: false,
          value: '',
        };
      }

      setKeyStates(states);
    } catch (err) {
      console.error('Failed to load key states:', err);
      // Initialize with empty states for browser dev mode
      const states: Record<string, { hasKey: boolean; isEditing: boolean; value: string }> = {};
      for (const key of API_KEYS) {
        states[key.id] = { hasKey: false, isEditing: false, value: '' };
      }
      setKeyStates(states);
    }
  }, []);

  useEffect(() => {
    loadKeyStates();
  }, [loadKeyStates]);

  const handleSaveKey = async (keyId: string) => {
    const state = keyStates[keyId];
    if (!state?.value.trim()) {
      setError('Please enter an API key');
      return;
    }

    setSaving(keyId);
    setError(null);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_api_key', { keyType: keyId, value: state.value.trim() });

      setKeyStates((prev) => ({
        ...prev,
        [keyId]: { hasKey: true, isEditing: false, value: '' },
      }));

      setSuccess(`${API_KEYS.find((k) => k.id === keyId)?.name} API key saved securely`);
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save API key');
    } finally {
      setSaving(null);
    }
  };

  const handleDeleteKey = async (keyId: string) => {
    setSaving(keyId);
    setError(null);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_api_key', { keyType: keyId });

      setKeyStates((prev) => ({
        ...prev,
        [keyId]: { hasKey: false, isEditing: false, value: '' },
      }));

      setSuccess(`${API_KEYS.find((k) => k.id === keyId)?.name} API key removed`);
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete API key');
    } finally {
      setSaving(null);
    }
  };

  const toggleEdit = (keyId: string) => {
    setKeyStates((prev) => ({
      ...prev,
      [keyId]: { ...prev[keyId], isEditing: !prev[keyId]?.isEditing, value: '' },
    }));
  };

  const updateValue = (keyId: string, value: string) => {
    setKeyStates((prev) => ({
      ...prev,
      [keyId]: { ...prev[keyId], value },
    }));
  };

  return (
    <main
      className="fixed inset-0 bg-voice-bg flex flex-col"
      style={{ background: '#0f0f23' }}
    >
      {/* Window drag region */}
      <div
        className="h-8 w-full flex-shrink-0 flex items-center justify-center"
        data-tauri-drag-region
      >
        <div className="w-12 h-1 bg-voice-border rounded-full" />
      </div>

      <div className="flex-1 overflow-y-auto">
        <div className="max-w-2xl mx-auto px-6 pb-12">
          {/* Header */}
          <div className="flex items-center justify-between mb-8">
            <div>
              <h1 className="text-2xl font-bold gradient-text">Settings</h1>
              <p className="text-sm text-gray-500 mt-1">
                Manage your API keys securely stored in the system keychain
              </p>
            </div>
            <Link
              href="/"
              className="px-4 py-2 text-sm text-gray-400 hover:text-white bg-voice-surface hover:bg-voice-border rounded-lg transition-colors"
            >
              Back
            </Link>
          </div>

        {/* Messages */}
        {error && (
          <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg">
            <p className="text-red-400 text-sm">{error}</p>
          </div>
        )}

        {success && (
          <div className="mb-4 p-3 bg-green-500/10 border border-green-500/20 rounded-lg">
            <p className="text-green-400 text-sm">{success}</p>
          </div>
        )}

        {/* API Keys */}
        <div className="space-y-4">
          {API_KEYS.map((keyConfig) => {
            const state = keyStates[keyConfig.id] || { hasKey: false, isEditing: false, value: '' };
            const isSaving = saving === keyConfig.id;

            return (
              <div
                key={keyConfig.id}
                className="p-4 bg-voice-surface border border-voice-border rounded-lg"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <h3 className="font-medium text-white">{keyConfig.name}</h3>
                      {state.hasKey && (
                        <span className="px-2 py-0.5 text-xs bg-green-500/20 text-green-400 rounded">
                          Configured
                        </span>
                      )}
                    </div>
                    <p className="text-sm text-gray-500 mt-1">{keyConfig.description}</p>
                  </div>

                  {!state.isEditing && (
                    <div className="flex gap-2">
                      <button
                        onClick={() => toggleEdit(keyConfig.id)}
                        className="px-3 py-1.5 text-sm text-gray-400 hover:text-white bg-voice-border hover:bg-voice-primary/20 rounded transition-colors"
                      >
                        {state.hasKey ? 'Update' : 'Add'}
                      </button>
                      {state.hasKey && (
                        <button
                          onClick={() => handleDeleteKey(keyConfig.id)}
                          disabled={isSaving}
                          className="px-3 py-1.5 text-sm text-red-400 hover:text-red-300 bg-red-500/10 hover:bg-red-500/20 rounded transition-colors disabled:opacity-50"
                        >
                          {isSaving ? 'Removing...' : 'Remove'}
                        </button>
                      )}
                    </div>
                  )}
                </div>

                {state.isEditing && (
                  <div className="mt-4 space-y-3">
                    <input
                      type="password"
                      value={state.value}
                      onChange={(e) => updateValue(keyConfig.id, e.target.value)}
                      placeholder={keyConfig.placeholder}
                      className="w-full px-3 py-2 bg-voice-bg border border-voice-border rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-voice-primary"
                      autoFocus
                    />
                    <div className="flex gap-2">
                      <button
                        onClick={() => handleSaveKey(keyConfig.id)}
                        disabled={isSaving || !state.value.trim()}
                        className="px-4 py-2 text-sm bg-voice-primary hover:bg-voice-secondary text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {isSaving ? 'Saving...' : 'Save to Keychain'}
                      </button>
                      <button
                        onClick={() => toggleEdit(keyConfig.id)}
                        disabled={isSaving}
                        className="px-4 py-2 text-sm text-gray-400 hover:text-white bg-voice-border rounded-lg transition-colors"
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>

          {/* Info */}
          <div className="mt-8 p-4 bg-voice-surface/50 border border-voice-border rounded-lg">
            <div className="flex items-start gap-3">
              <LockIcon className="w-5 h-5 text-voice-primary mt-0.5" />
              <div>
                <h4 className="font-medium text-white">Secure Storage</h4>
                <p className="text-sm text-gray-500 mt-1">
                  API keys are stored in your operating system&apos;s secure keychain (macOS Keychain
                  / Windows Credential Manager). They never leave your device and are encrypted at
                  rest.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Resize handle */}
      <WindowControls />
    </main>
  );
}

function LockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
      />
    </svg>
  );
}
