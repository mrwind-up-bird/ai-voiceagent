'use client';

import { useEffect, useState, useCallback } from 'react';
import {
  getPersonaPreference,
  setPersonaPreference,
  clearPersonaPreference,
  type PersonaPreference,
} from '../lib/personaPreference';

interface PersonaSummary {
  id: string;
  name: string;
  description?: string | null;
  category?: string | null;
  isLead?: boolean;
}

interface PersonaCircle {
  id: string;
  slug: string;
  name: string;
  tagline?: string | null;
  description?: string | null;
  personas: PersonaSummary[];
}

/**
 * Sub-Project E — Settings widget for choosing which Persona Studio
 * persona Aurus should use to rephrase outputs (todos, brain dumps,
 * letters). Loads the catalogue lazily via the `list_personas`
 * Tauri command; surfaces a helpful empty state when the
 * persona_studio token is missing.
 */
export function PersonaSelector() {
  const [circles, setCircles] = useState<PersonaCircle[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pref, setPref] = useState<PersonaPreference>({
    personaId: null,
    circleId: null,
  });

  useEffect(() => {
    setPref(getPersonaPreference());
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<PersonaCircle[]>('list_personas');
      setCircles(result);
      if (result.length === 0) {
        setError(
          'Persona Studio returned no personas for this token. Check your Persona Studio access.',
        );
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  const handleSelect = (circleId: string, personaId: string | null) => {
    const next = { circleId, personaId };
    setPref(next);
    setPersonaPreference(next);
  };

  const handleClear = () => {
    setPref({ personaId: null, circleId: null });
    clearPersonaPreference();
  };

  return (
    <div className="glass rounded-lg p-4">
      <div className="flex items-center justify-between mb-3">
        <div>
          <h3 className="text-sm font-medium text-white">Persona Voice</h3>
          <p className="text-xs text-gray-400 mt-0.5">
            Apply a Persona Studio voice to todos, brain dumps, and letters.
          </p>
        </div>
        <button
          type="button"
          onClick={load}
          disabled={loading}
          className="px-3 py-1 text-xs rounded bg-white/10 hover:bg-white/20 transition disabled:opacity-50"
        >
          {loading ? 'Loading…' : circles.length > 0 ? 'Refresh' : 'Load personas'}
        </button>
      </div>

      {error && (
        <div className="mb-3 p-2 bg-red-500/10 border border-red-500/20 rounded text-xs text-red-300">
          {error}
        </div>
      )}

      {circles.length > 0 && (
        <div className="space-y-3">
          {circles.map((circle) => (
            <div key={circle.id}>
              <div className="text-xs uppercase tracking-wider text-gray-500 mb-1">
                {circle.name}
                {circle.tagline ? ` — ${circle.tagline}` : ''}
              </div>
              <div className="flex flex-wrap gap-1.5">
                {circle.personas.map((p) => {
                  const selected = pref.personaId === p.id;
                  return (
                    <button
                      key={p.id}
                      type="button"
                      onClick={() => handleSelect(circle.id, p.id)}
                      className={`text-xs px-2 py-1 rounded border transition ${
                        selected
                          ? 'bg-amber-500/30 border-amber-400 text-amber-100'
                          : 'bg-white/5 border-white/10 text-gray-300 hover:bg-white/10'
                      }`}
                      title={p.description ?? p.category ?? ''}
                    >
                      {p.name}
                      {p.isLead ? ' ★' : ''}
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
          {pref.personaId && (
            <button
              type="button"
              onClick={handleClear}
              className="text-xs text-gray-500 hover:text-white underline mt-2"
            >
              Clear selection (no persona-tuning)
            </button>
          )}
        </div>
      )}

      {!loading && circles.length === 0 && !error && (
        <p className="text-xs text-gray-500">
          Add your Persona Studio token above, then click <em>Load personas</em>.
        </p>
      )}
    </div>
  );
}
