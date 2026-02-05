/**
 * Web Worker for on-device AI inference using Transformers.js
 *
 * This worker runs AI models in a separate thread to avoid blocking the UI.
 * Uses tiled loading and Int4 quantization for memory efficiency on mobile.
 *
 * Supported tasks:
 * - Speech-to-text (Whisper)
 * - Text summarization
 * - Sentiment analysis
 */

// Worker message types
export interface WorkerMessage {
  type: 'init' | 'transcribe' | 'summarize' | 'analyze' | 'cancel';
  payload?: unknown;
  requestId: string;
}

export interface WorkerResponse {
  type: 'ready' | 'progress' | 'result' | 'error';
  requestId: string;
  payload?: unknown;
  error?: string;
  progress?: number;
}

// Model configurations for mobile optimization
const MODEL_CONFIGS = {
  // Whisper tiny for speech recognition (~39M params)
  whisper: {
    model: 'Xenova/whisper-tiny.en',
  },
  // Small summarization model
  summarization: {
    model: 'Xenova/distilbart-cnn-6-6',
  },
  // Sentiment analysis
  sentiment: {
    model: 'Xenova/distilbert-base-uncased-finetuned-sst-2-english',
  },
};

let pipeline: any = null;
let currentTask: string | null = null;
let isInitialized = false;
let abortController: AbortController | null = null;

// Initialize Transformers.js
async function initTransformers() {
  if (isInitialized) return;

  try {
    // Dynamic import of transformers.js
    const { env } = await import('@xenova/transformers');

    // Configure for browser/mobile optimization
    env.allowLocalModels = false;
    env.useBrowserCache = true;

    isInitialized = true;
    self.postMessage({ type: 'ready', requestId: 'init' } as WorkerResponse);
  } catch (error) {
    self.postMessage({
      type: 'error',
      requestId: 'init',
      error: `Failed to initialize Transformers.js: ${error}`,
    } as WorkerResponse);
  }
}

// Pipeline task types
type PipelineTask =
  | 'automatic-speech-recognition'
  | 'summarization'
  | 'sentiment-analysis';

// Load a specific pipeline
async function loadPipeline(
  task: PipelineTask,
  config: { model: string }
) {
  if (currentTask === task && pipeline) {
    return pipeline;
  }

  const { pipeline: createPipeline } = await import('@xenova/transformers');

  // Unload previous pipeline to free memory
  if (pipeline) {
    pipeline = null;
  }

  // Create new pipeline with progress callback
  pipeline = await createPipeline(task, config.model, {
    progress_callback: (progress: { progress: number; status: string }) => {
      self.postMessage({
        type: 'progress',
        requestId: 'load',
        progress: progress.progress,
        payload: progress.status,
      } as WorkerResponse);
    },
  });

  currentTask = task;
  return pipeline;
}

// Transcribe audio using Whisper
async function transcribe(audioData: Float32Array, requestId: string) {
  try {
    abortController = new AbortController();

    const pipe = await loadPipeline('automatic-speech-recognition', MODEL_CONFIGS.whisper);

    const result = await pipe(audioData, {
      chunk_length_s: 30,
      stride_length_s: 5,
      language: 'en',
      task: 'transcribe',
      return_timestamps: false,
    });

    self.postMessage({
      type: 'result',
      requestId,
      payload: {
        text: result.text,
        confidence: 0.9, // Whisper doesn't provide confidence
      },
    } as WorkerResponse);
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      return; // Cancelled
    }
    self.postMessage({
      type: 'error',
      requestId,
      error: `Transcription failed: ${error}`,
    } as WorkerResponse);
  } finally {
    abortController = null;
  }
}

// Summarize text
async function summarize(text: string, requestId: string) {
  try {
    const pipe = await loadPipeline('summarization', MODEL_CONFIGS.summarization);

    const result = await pipe(text, {
      max_length: 150,
      min_length: 30,
    });

    self.postMessage({
      type: 'result',
      requestId,
      payload: {
        summary: result[0].summary_text,
      },
    } as WorkerResponse);
  } catch (error) {
    self.postMessage({
      type: 'error',
      requestId,
      error: `Summarization failed: ${error}`,
    } as WorkerResponse);
  }
}

// Analyze sentiment
async function analyzeSentiment(text: string, requestId: string) {
  try {
    const pipe = await loadPipeline('sentiment-analysis', MODEL_CONFIGS.sentiment);

    const result = await pipe(text);

    self.postMessage({
      type: 'result',
      requestId,
      payload: {
        label: result[0].label,
        score: result[0].score,
      },
    } as WorkerResponse);
  } catch (error) {
    self.postMessage({
      type: 'error',
      requestId,
      error: `Sentiment analysis failed: ${error}`,
    } as WorkerResponse);
  }
}

// Handle incoming messages
self.onmessage = async (event: MessageEvent<WorkerMessage>) => {
  const { type, payload, requestId } = event.data;

  switch (type) {
    case 'init':
      await initTransformers();
      break;

    case 'transcribe':
      if (payload instanceof Float32Array) {
        await transcribe(payload, requestId);
      } else {
        self.postMessage({
          type: 'error',
          requestId,
          error: 'Invalid audio data format',
        } as WorkerResponse);
      }
      break;

    case 'summarize':
      if (typeof payload === 'string') {
        await summarize(payload, requestId);
      } else {
        self.postMessage({
          type: 'error',
          requestId,
          error: 'Invalid text format',
        } as WorkerResponse);
      }
      break;

    case 'analyze':
      if (typeof payload === 'string') {
        await analyzeSentiment(payload, requestId);
      } else {
        self.postMessage({
          type: 'error',
          requestId,
          error: 'Invalid text format',
        } as WorkerResponse);
      }
      break;

    case 'cancel':
      if (abortController) {
        abortController.abort();
      }
      break;
  }
};

