# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aurus Voice Intelligence is a native desktop voice assistant built with Tauri v2 + Next.js 14. It features a Spotlight-like interface triggered by global hotkey (Cmd+Shift+V) with three AI agents for processing voice transcripts.

## Build & Development Commands

```bash
# Frontend development (Next.js)
pnpm dev              # Start Next.js dev server on localhost:3000
pnpm build            # Build static export to ./out directory

# Tauri desktop app
pnpm tauri dev        # Run Tauri app with hot reload (starts Next.js automatically)
pnpm tauri build      # Build production desktop binary

# Testing
pnpm test             # Run Vitest unit tests
pnpm test -- --watch  # Run tests in watch mode
pnpm test -- agents.test.ts  # Run single test file
pnpm test:e2e         # Run Playwright E2E tests (starts dev server)

# Rust backend only
cd src-tauri && cargo build      # Build Rust backend
cd src-tauri && cargo check      # Type check without building
cd src-tauri && cargo clippy     # Lint Rust code
```

## Architecture

### Stack Separation

**Rust Backend (src-tauri/)** handles all system-level operations:
- Audio capture via CPAL (native sample rate → resampled to 16kHz mono)
- Direct audio streaming to Deepgram WebSocket (bypasses frontend)
- VAD (Voice Activity Detection) with energy-based threshold
- API calls to external services (Deepgram, AssemblyAI, OpenAI, Anthropic)
- Secrets management via OS keychain (macOS Keychain / Windows Credential Manager)
- Local Whisper inference for offline transcription

**Next.js Frontend (app/)** is a static export consumed by Tauri:
- React components for UI
- Zustand store for state management
- Tauri event listeners for real-time updates
- No audio processing (handled entirely in Rust)

### Audio Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│ Rust (audio.rs)                                                 │
│ ┌─────────────┐    ┌────────────┐    ┌───────────────────────┐  │
│ │ CPAL Input  │ -> │ Mono Conv  │ -> │ Resample to 16kHz     │  │
│ │ (48kHz/etc) │    │ + VAD      │    │ (if needed)           │  │
│ └─────────────┘    └────────────┘    └───────────┬───────────┘  │
│                                                   │              │
│                                     ┌─────────────▼───────────┐  │
│                                     │ transcription.rs        │  │
│                                     │ Direct send to Deepgram │  │
│                                     │ WebSocket (linear16)    │  │
│                                     └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

Audio is sent directly from Rust to Deepgram without passing through the frontend, avoiding JSON serialization overhead and potential data corruption.

### Rust Module Structure

```
src-tauri/src/
├── lib.rs           # Tauri app setup, global shortcut, command registration
├── audio.rs         # CPAL audio capture, VAD, resampling, direct Deepgram forwarding
├── transcription.rs # Deepgram WebSocket streaming, AssemblyAI batch, local Whisper
├── secrets.rs       # API key storage/retrieval via OS keychain
└── agents/
    ├── mod.rs           # Module exports
    ├── action_items.rs  # GPT-4o structured JSON extraction
    ├── tone_shifter.rs  # Claude Sonnet streaming with 8 tone types
    └── music_matcher.rs # Q-Records API + OpenAI mood analysis
```

### Frontend Structure

```
app/
├── page.tsx                    # Main voice interface
├── settings/page.tsx           # API key management
├── store/voiceStore.ts         # Zustand state (recording, transcript, agent results)
├── hooks/
│   ├── useTauriEvents.ts       # Listens to all Tauri backend events
│   ├── useDeepgramStreaming.ts # Audio forwarding disabled (handled in Rust)
│   └── useGlobalShortcut.ts    # Escape key handling
└── components/
    ├── VoiceInput.tsx          # Record button with waveform visualization
    ├── TranscriptDisplay.tsx   # Real-time transcript with interim text
    ├── AgentSelector.tsx       # Three agent buttons
    └── AgentResults.tsx        # Dynamic results display per agent
```

### Tauri Commands (invoke from frontend)

**Audio:** `start_recording`, `stop_recording`, `is_recording`, `list_audio_devices`

**Transcription:** `start_deepgram_stream`, `stop_deepgram_stream`, `send_audio_to_deepgram`, `is_deepgram_streaming`, `transcribe_with_assemblyai`, `transcribe_local_whisper`

**Agents:**
- `extract_action_items` / `extract_action_items_streaming` - GPT-4o action extraction
- `shift_tone` / `shift_tone_streaming` - Claude tone rewriting
- `get_available_tones` - List of 8 tone types
- `match_music` - Q-Records music search
- `analyze_mood_from_transcript` - OpenAI mood analysis
- `get_available_moods`, `get_available_genres` - Music metadata

**Secrets:** `set_api_key`, `get_api_key`, `delete_api_key`, `has_api_keys`

### Tauri Events (emitted to frontend)

**Audio:** `recording-started`, `recording-stopped`, `recording-error`, `audio-chunk`, `vad-event`

**Transcription:** `deepgram-connected`, `transcript`

**Agents:**
- `action-items-extracted`, `action-items-processing`, `action-items-complete`
- `tone-shifted`, `tone-shift-started`, `tone-shift-chunk`, `tone-shift-complete`
- `music-matched`, `mood-analyzed`

## Key Constraints

- Next.js must use `output: 'export'` (static files only, no server actions)
- All sensitive operations (API calls, secrets) must be in Rust, never in frontend
- Audio is captured at native sample rate and resampled to 16kHz mono in Rust
- Audio goes directly from Rust to Deepgram (no frontend round-trip)
- Window is frameless, transparent, and always-on-top (Spotlight-style)
- macOS requires NSMicrophoneUsageDescription in Info.plist

## API Configuration

### Deepgram (Primary Transcription)
- Uses Nova-2 model with streaming WebSocket
- Language: German (`language=de`)
- Parameters: `encoding=linear16`, `sample_rate=16000`, `channels=1`
- Features: `interim_results`, `punctuate`, `smart_format`, `endpointing=300`

### Claude (Tone Shifter)
- Model: `claude-sonnet-4-20250514`
- Supports streaming responses via SSE

### OpenAI (Action Items + Mood Analysis)
- Action Items: `gpt-4o` with JSON response format
- Mood Analysis: `gpt-4o-mini` for cost efficiency

## Testing

- Unit tests mock Tauri APIs in `__tests__/setup.ts`
- E2E tests run against Next.js dev server (not full Tauri app)
- Test files: `__tests__/*.test.ts` for unit, `e2e/*.spec.ts` for E2E
- All 33 tests passing (12 agent tests, 21 component tests)
