'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import type { WorkerMessage, WorkerResponse } from '../workers/ai-worker';

interface LocalAIState {
  isReady: boolean;
  isLoading: boolean;
  progress: number;
  error: string | null;
}

interface TranscriptionResult {
  text: string;
  confidence: number;
}

interface SummarizationResult {
  summary: string;
}

interface SentimentResult {
  label: 'POSITIVE' | 'NEGATIVE';
  score: number;
}

/**
 * Hook for using on-device AI capabilities via Web Worker.
 * Provides transcription, summarization, and sentiment analysis.
 *
 * Uses Transformers.js with WebGPU acceleration when available.
 */
export function useLocalAI() {
  const [state, setState] = useState<LocalAIState>({
    isReady: false,
    isLoading: false,
    progress: 0,
    error: null,
  });

  const workerRef = useRef<Worker | null>(null);
  const pendingRequests = useRef<Map<string, (result: unknown) => void>>(new Map());
  const pendingErrors = useRef<Map<string, (error: Error) => void>>(new Map());

  // Initialize worker
  useEffect(() => {
    // Only run in browser
    if (typeof window === 'undefined') return;

    // Check if Worker is supported
    if (!window.Worker) {
      setState((prev) => ({
        ...prev,
        error: 'Web Workers not supported in this browser',
      }));
      return;
    }

    // Create worker
    try {
      workerRef.current = new Worker(
        new URL('../workers/ai-worker.ts', import.meta.url),
        { type: 'module' }
      );

      // Handle messages from worker
      workerRef.current.onmessage = (event: MessageEvent<WorkerResponse>) => {
        const { type, requestId, payload, error, progress } = event.data;

        switch (type) {
          case 'ready':
            setState((prev) => ({ ...prev, isReady: true, isLoading: false }));
            break;

          case 'progress':
            setState((prev) => ({ ...prev, progress: progress || 0 }));
            break;

          case 'result':
            const resolve = pendingRequests.current.get(requestId);
            if (resolve) {
              resolve(payload);
              pendingRequests.current.delete(requestId);
              pendingErrors.current.delete(requestId);
            }
            setState((prev) => ({ ...prev, isLoading: false, progress: 0 }));
            break;

          case 'error':
            const reject = pendingErrors.current.get(requestId);
            if (reject) {
              reject(new Error(error));
              pendingRequests.current.delete(requestId);
              pendingErrors.current.delete(requestId);
            }
            setState((prev) => ({
              ...prev,
              isLoading: false,
              progress: 0,
              error: error || 'Unknown error',
            }));
            break;
        }
      };

      workerRef.current.onerror = (error) => {
        console.error('AI Worker error:', error);
        setState((prev) => ({
          ...prev,
          error: `Worker error: ${error.message}`,
        }));
      };

      // Initialize transformers.js
      workerRef.current.postMessage({
        type: 'init',
        requestId: 'init',
      } as WorkerMessage);

      setState((prev) => ({ ...prev, isLoading: true }));
    } catch (error) {
      console.error('Failed to create AI worker:', error);
      setState((prev) => ({
        ...prev,
        error: `Failed to create worker: ${error}`,
      }));
    }

    // Cleanup
    return () => {
      workerRef.current?.terminate();
      workerRef.current = null;
    };
  }, []);

  // Generate unique request ID
  const generateRequestId = useCallback(() => {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }, []);

  // Send message to worker and wait for response
  const sendRequest = useCallback(
    <T>(message: Omit<WorkerMessage, 'requestId'>): Promise<T> => {
      return new Promise((resolve, reject) => {
        if (!workerRef.current) {
          reject(new Error('AI worker not initialized'));
          return;
        }

        const requestId = generateRequestId();
        pendingRequests.current.set(requestId, resolve as (result: unknown) => void);
        pendingErrors.current.set(requestId, reject);

        setState((prev) => ({ ...prev, isLoading: true, error: null }));

        workerRef.current.postMessage({
          ...message,
          requestId,
        } as WorkerMessage);
      });
    },
    [generateRequestId]
  );

  // Transcribe audio to text
  const transcribe = useCallback(
    async (audioData: Float32Array): Promise<TranscriptionResult> => {
      return sendRequest<TranscriptionResult>({
        type: 'transcribe',
        payload: audioData,
      });
    },
    [sendRequest]
  );

  // Summarize text
  const summarize = useCallback(
    async (text: string): Promise<SummarizationResult> => {
      return sendRequest<SummarizationResult>({
        type: 'summarize',
        payload: text,
      });
    },
    [sendRequest]
  );

  // Analyze sentiment
  const analyzeSentiment = useCallback(
    async (text: string): Promise<SentimentResult> => {
      return sendRequest<SentimentResult>({
        type: 'analyze',
        payload: text,
      });
    },
    [sendRequest]
  );

  // Cancel current operation
  const cancel = useCallback(() => {
    if (workerRef.current) {
      workerRef.current.postMessage({
        type: 'cancel',
        requestId: 'cancel',
      } as WorkerMessage);
    }
  }, []);

  return {
    ...state,
    transcribe,
    summarize,
    analyzeSentiment,
    cancel,
  };
}

/**
 * Check if local AI is available on this device.
 * Returns true if WebGPU or WASM is supported.
 */
export function useLocalAIAvailable(): boolean {
  const [isAvailable, setIsAvailable] = useState(false);

  useEffect(() => {
    async function check() {
      // Check for Worker support
      if (typeof Worker === 'undefined') {
        setIsAvailable(false);
        return;
      }

      // Check for WebGPU (preferred) or WebAssembly (fallback)
      const hasWebGPU = 'gpu' in navigator;
      const hasWasm = typeof WebAssembly !== 'undefined';

      setIsAvailable(hasWebGPU || hasWasm);
    }

    check();
  }, []);

  return isAvailable;
}
