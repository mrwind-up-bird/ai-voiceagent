'use client';

import { useEffect, useCallback } from 'react';

interface ShortcutConfig {
  key: string;
  modifiers: {
    meta?: boolean;
    shift?: boolean;
    ctrl?: boolean;
    alt?: boolean;
  };
  callback: () => void;
}

export function useGlobalShortcut(config: ShortcutConfig) {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const { key, modifiers, callback } = config;

      const metaMatch = modifiers.meta ? event.metaKey : !event.metaKey;
      const shiftMatch = modifiers.shift ? event.shiftKey : !event.shiftKey;
      const ctrlMatch = modifiers.ctrl ? event.ctrlKey : !event.ctrlKey;
      const altMatch = modifiers.alt ? event.altKey : !event.altKey;

      if (
        event.key.toLowerCase() === key.toLowerCase() &&
        metaMatch &&
        shiftMatch &&
        ctrlMatch &&
        altMatch
      ) {
        event.preventDefault();
        callback();
      }
    },
    [config]
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}

// Convenience hook for common shortcuts
export function useEscapeKey(callback: () => void) {
  useGlobalShortcut({
    key: 'Escape',
    modifiers: {},
    callback,
  });
}

export function useRecordingShortcut(callback: () => void) {
  useGlobalShortcut({
    key: 'r',
    modifiers: { meta: true },
    callback,
  });
}
