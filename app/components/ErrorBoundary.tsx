'use client';

import React from 'react';

interface ErrorBoundaryProps {
  children: React.ReactNode;
  /** Optional custom fallback. If omitted, a generic recovery UI is shown. */
  fallback?: (error: Error, reset: () => void) => React.ReactNode;
  /** Optional callback for telemetry. Receives the error and component stack. */
  onError?: (error: Error, componentStack: string | null) => void;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * Top-level React error boundary (H9). Without this, any uncaught
 * render exception in a child component would unmount the entire app
 * tree, leaving a blank window with no recovery path — especially bad
 * on iOS where users cannot easily reopen a frozen WebView.
 */
export class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo): void {
    if (this.props.onError) {
      this.props.onError(error, info.componentStack ?? null);
    }
    if (process.env.NODE_ENV === 'development') {
      // eslint-disable-next-line no-console
      console.error('ErrorBoundary caught:', error, info);
    }
  }

  reset = (): void => {
    this.setState({ error: null });
  };

  render(): React.ReactNode {
    const { error } = this.state;
    if (!error) {
      return this.props.children;
    }
    if (this.props.fallback) {
      return this.props.fallback(error, this.reset);
    }
    return (
      <div
        role="alert"
        className="flex flex-col items-center justify-center min-h-screen p-8 text-center bg-black/80 text-white"
      >
        <h1 className="text-xl font-semibold mb-2">Something went wrong</h1>
        <p className="text-sm opacity-80 mb-4 max-w-md">
          The interface hit an unexpected error. Your recording state and
          settings are still intact — tap below to reset the view.
        </p>
        <pre className="text-xs opacity-60 mb-4 max-w-md overflow-auto">
          {error.message}
        </pre>
        <button
          type="button"
          onClick={this.reset}
          className="px-4 py-2 rounded bg-white/10 hover:bg-white/20 transition"
        >
          Reset view
        </button>
      </div>
    );
  }
}
