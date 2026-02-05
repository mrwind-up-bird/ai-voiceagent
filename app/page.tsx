'use client';

import { useEffect } from 'react';
import Link from 'next/link';
import { VoiceInput } from './components/VoiceInput';
import { WindowControls } from './components/WindowControls';
import { TranscriptDisplay } from './components/TranscriptDisplay';
import { AgentSelector } from './components/AgentSelector';
import { AgentResults } from './components/AgentResults';
import { TranslationPanel } from './components/TranslationPanel';
import { ToneSelector } from './components/ToneSelector';
import { ResizeHandle } from './components/ResizeHandle';
import { useTauriEvents } from './hooks/useTauriEvents';
import { useAudioForwarding } from './hooks/useDeepgramStreaming';
import { useEscapeKey } from './hooks/useGlobalShortcut';
import { useVoiceStore } from './store/voiceStore';
import { usePlatform } from './hooks/usePlatform';

export default function Home() {
  const { error, setError, reset } = useVoiceStore();
  const { isDesktop, supportsWindowControls, supportsKeyboardShortcuts } = usePlatform();

  useTauriEvents();
  useAudioForwarding();

  // Desktop-only: hide window on Escape
  useEscapeKey(async () => {
    if (!isDesktop) return;
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const window = getCurrentWindow();
      await window.hide();
    } catch {
      // Running in browser, ignore
    }
  });

  useEffect(() => {
    if (error) {
      const timer = setTimeout(() => setError(null), 5000);
      return () => clearTimeout(timer);
    }
  }, [error, setError]);

  return (
    <main className="h-screen w-full flex justify-center p-4 overflow-hidden bg-transparent">
      <div className="
        spotlight-container
        relative
        overflow-hidden
        w-full
        max-w-3xl
        h-full
        flex
        flex-col
        bg-black/60
        backdrop-blur-xl
        rounded-xl
        border
        border-white/10
        shadow-2xl

        before:content-['']
        before:absolute
        before:inset-0
        before:z-0
        before:opacity-10
        before:bg-[url('/aurus-logo.png')]
        before:bg-cover
        before:bg-center
        before:bg-no-repeat
        before:pointer-events-none
      ">
        <div
          className="relative z-10 flex items-center justify-between px-6 py-2 flex-shrink-0"
          data-tauri-drag-region
        >
          <div className="flex items-center justify-space-between gap-3">
            <h3 className="text-xs text-gray-500">
              <span className="bg-[url('/aurus-logo.png')] max-h-m"></span>
              Aurus Voice Intelligence
            </h3>
            <button
              onClick={reset}
              className="text-xs text-gray-500 hover:text-gray-300 transition-colors"
            >
              Clear
            </button>
            <Link
              href="/settings"
              className="p-1.5 text-gray-500 hover:text-gray-300 transition-colors rounded-lg hover:bg-white/5"
              title="Settings"
            >
              <SettingsIcon className="w-4 h-4" />
            </Link>
          </div>
        </div>

        {/* Content area - scrollable */}
        <div className="relative z-10 px-6 py-4 flex-1 overflow-y-auto overflow-x-hidden">
          {error && (
            <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg">
              <p className="text-red-400 text-sm">{error}</p>
            </div>
          )}

          <div className="flex flex-col items-center gap-6">
            <VoiceInput />
            <TranscriptDisplay />
            <AgentSelector />
            <TranslationPanel />
            <ToneSelector />
            <AgentResults />
          </div>
        </div>

        {/* Footer - show keyboard shortcut hint only on desktop */}
        {supportsKeyboardShortcuts && (
          <div className="relative z-10 px-6 py-3 text-center border-t border-white/5 flex-shrink-0">
            <p className="text-xs text-gray-600">
              Press <kbd className="px-1.5 py-0.5 bg-white/5 rounded text-gray-400">Cmd+Shift+V</kbd> to toggle
            </p>
          </div>
        )}

        {/* Resize handles for frameless window - desktop only */}
        {supportsWindowControls && <ResizeHandle />}
      </div>

      {/* Window controls - desktop only */}
      {supportsWindowControls && <WindowControls />}
    </main>
  );
}

function SettingsIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
      <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    </svg>
  );
}