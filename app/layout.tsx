'use client';

import './globals.css';
import { useEffect } from 'react';
import { ErrorBoundary } from './components/ErrorBoundary';
import { useTauriEvents } from './hooks/useTauriEvents';
import { useAudioForwarding } from './hooks/useDeepgramStreaming';

/**
 * GlobalListeners mounts the Tauri event hook + audio forwarding at
 * the layout level (H6) so navigating between `/` and `/settings`
 * does NOT unmount the listeners and orphan in-flight transcripts.
 * Without this, every "click gear, edit key, click back" mid-thought
 * silently dropped any words spoken during the transit.
 */
function GlobalListeners() {
  useTauriEvents();
  useAudioForwarding();
  return null;
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  useEffect(() => {
    // Prevent context menu in production
    if (process.env.NODE_ENV === 'production') {
      document.addEventListener('contextmenu', (e) => e.preventDefault());
    }

    // Prevent text selection dragging (for native feel)
    document.addEventListener('selectstart', (e) => {
      if ((e.target as HTMLElement).tagName !== 'INPUT' &&
          (e.target as HTMLElement).tagName !== 'TEXTAREA') {
        e.preventDefault();
      }
    });
  }, []);

  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>Voice Intelligence Hub</title>
      </head>
      <body className="bg-transparent">
        <ErrorBoundary>
          <GlobalListeners />
          {children}
        </ErrorBoundary>
      </body>
    </html>
  );
}
