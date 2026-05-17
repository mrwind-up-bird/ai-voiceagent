'use client';

import { useCallback, useEffect, useState } from 'react';

const DEFAULT_HINT = 'http://localhost:3000';

/**
 * Settings widget for the nyxCore base URL. Persists into the
 * `nyxcore_base_url` keychain slot via the generic set_api_key /
 * delete_api_key Tauri commands.
 *
 * Why this isn't auto-detected: Aurus' own `pnpm tauri dev` runs
 * Next.js on :3000 by default, which collides with the nyxcore-systems
 * sibling project's default. Users running both locally MUST set
 * either of them to a non-3000 port (typical: nyxcore-systems on
 * 3001 via `PORT=3001 pnpm dev`).
 */
export function NyxCoreBaseUrlInput() {
  const [stored, setStored] = useState<string | null>(null);
  const [value, setValue] = useState('');
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const existing = await invoke<string | null>('get_api_key', {
        keyType: 'nyxcore_base_url',
      });
      setStored(existing);
    } catch {
      setStored(null);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const save = async () => {
    const trimmed = value.trim();
    if (!trimmed) {
      setError('Enter a URL like http://localhost:3001');
      return;
    }
    if (!/^https?:\/\//.test(trimmed)) {
      setError('URL must start with http:// or https://');
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_api_key', {
        keyType: 'nyxcore_base_url',
        value: trimmed,
      });
      setStored(trimmed);
      setEditing(false);
      setValue('');
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const clear = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_api_key', { keyType: 'nyxcore_base_url' });
      setStored(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="glass rounded-lg p-4">
      <div className="flex items-start justify-between gap-3 mb-2">
        <div>
          <h3 className="text-sm font-medium text-white">nyxCore Base URL</h3>
          <p className="text-xs text-gray-400 mt-0.5">
            Where to reach <code>nyxcore-systems</code>. Default is{' '}
            <code>{DEFAULT_HINT}</code> — but Aurus' own dev server also runs there,
            so if you're running both locally you need a different port for one of
            them (e.g. <code>PORT=3001 pnpm dev</code> in nyxcore-systems).
          </p>
        </div>
        {stored && !editing && (
          <span className="shrink-0 px-2 py-0.5 text-[10px] uppercase tracking-wider rounded bg-green-500/20 text-green-400 border border-green-500/30">
            Set
          </span>
        )}
      </div>

      {editing ? (
        <div className="flex gap-2 items-center">
          <input
            type="url"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="http://localhost:3001"
            className="flex-1 px-3 py-1.5 text-sm bg-black/30 border border-voice-border rounded text-white placeholder-gray-500 focus:outline-none focus:border-voice-primary"
            autoFocus
          />
          <button
            type="button"
            onClick={save}
            disabled={saving}
            className="px-3 py-1.5 text-xs rounded bg-voice-primary/20 hover:bg-voice-primary/30 text-voice-primary border border-voice-primary/40 transition disabled:opacity-50"
          >
            {saving ? 'Saving…' : 'Save'}
          </button>
          <button
            type="button"
            onClick={() => {
              setEditing(false);
              setValue('');
              setError(null);
            }}
            className="px-3 py-1.5 text-xs rounded bg-white/5 hover:bg-white/10 text-gray-400 transition"
          >
            Cancel
          </button>
        </div>
      ) : (
        <div className="flex gap-2 items-center">
          <code className="flex-1 px-3 py-1.5 text-sm bg-black/30 border border-voice-border/50 rounded text-gray-300 font-mono">
            {stored ?? `${DEFAULT_HINT} (default)`}
          </code>
          <button
            type="button"
            onClick={() => {
              setValue(stored ?? DEFAULT_HINT);
              setEditing(true);
              setError(null);
            }}
            className="px-3 py-1.5 text-xs rounded bg-white/5 hover:bg-white/10 text-gray-300 transition"
          >
            {stored ? 'Update' : 'Set'}
          </button>
          {stored && (
            <button
              type="button"
              onClick={clear}
              className="px-3 py-1.5 text-xs rounded bg-red-500/10 hover:bg-red-500/20 text-red-400 transition"
            >
              Reset
            </button>
          )}
        </div>
      )}

      {error && (
        <div className="mt-2 p-2 bg-red-500/10 border border-red-500/20 rounded text-xs text-red-300">
          {error}
        </div>
      )}
    </div>
  );
}
