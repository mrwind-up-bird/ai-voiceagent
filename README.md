# Aurus Voice Intelligence

A native desktop voice assistant built with Tauri v2 + Next.js 14. Features a Spotlight-like interface with real-time voice transcription and AI-powered text processing agents.

## Features

### Voice Transcription
- Real-time streaming transcription via Deepgram Nova-2
- Voice Activity Detection (VAD) with visual feedback
- Support for German language transcription
- Audio recording export to WAV format

### AI Agents

| Agent | Description | API |
|-------|-------------|-----|
| **Translator** | Translate between 12+ languages with auto-detect | OpenAI GPT-4o |
| **Tone Shifter** | Rewrite text in 8 different tones with adjustable intensity | Claude Sonnet |
| **Action Items** | Extract tasks and commitments from transcripts | OpenAI GPT-4o |
| **Music Matcher** | Find matching music based on transcript mood | Q-Records + OpenAI |

### Additional Features
- **Copy to Clipboard** - One-click copy for all outputs
- **Text-to-Speech** - Native TTS to read results aloud (macOS/Windows)
- **Tone Presets** - Visual preset cards with examples and use cases
- **Intensity Control** - Slider (1-10) for tone shift strength
- **Resizable Window** - Drag to resize with min/max constraints

## Installation

### Prerequisites
- Node.js 18+
- pnpm
- Rust (latest stable)
- Xcode Command Line Tools (macOS)

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/ai-voiceagent.git
cd ai-voiceagent

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

## API Keys

The app stores API keys securely in the OS keychain (macOS Keychain / Windows Credential Manager).

Configure keys in the Settings page (`/settings`):

| Service | Required For |
|---------|--------------|
| Deepgram | Voice transcription |
| OpenAI | Translator, Action Items, Music mood analysis |
| Anthropic | Tone Shifter |
| Q-Records | Music Matcher (optional) |

## Architecture

### Stack Separation

**Rust Backend (`src-tauri/`)** handles all system-level operations:
- Audio capture via CPAL (native sample rate → resampled to 16kHz mono)
- Direct audio streaming to Deepgram WebSocket
- VAD with energy-based threshold
- API calls to external services
- Secrets management via OS keychain
- Native Text-to-Speech

**Next.js Frontend (`app/`)** is a static export consumed by Tauri:
- React components for UI
- Zustand store for state management
- Tauri event listeners for real-time updates

### Audio Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│ Rust (audio.rs)                                                 │
│ ┌─────────────┐    ┌────────────┐    ┌───────────────────────┐  │
│ │ CPAL Input  │ -> │ Mono Conv  │ -> │ Resample to 16kHz     │  │
│ │ (48kHz/etc) │    │ + VAD      │    │ + Buffer for export   │  │
│ └─────────────┘    └────────────┘    └───────────┬───────────┘  │
│                                                   │              │
│                                     ┌─────────────▼───────────┐  │
│                                     │ transcription.rs        │  │
│                                     │ Direct send to Deepgram │  │
│                                     │ WebSocket (linear16)    │  │
│                                     └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
ai-voiceagent/
├── app/                          # Next.js frontend
│   ├── components/               # React components
│   │   ├── VoiceInput.tsx        # Record button + waveform
│   │   ├── TranscriptDisplay.tsx # Real-time transcript
│   │   ├── AgentSelector.tsx     # Agent buttons
│   │   ├── AgentResults.tsx      # Dynamic results display
│   │   ├── ToneSelector.tsx      # Tone preset cards + intensity
│   │   ├── TranslationPanel.tsx  # Language selection
│   │   └── ResizeHandle.tsx      # Window resize handles
│   ├── hooks/                    # Custom React hooks
│   │   ├── useTauriEvents.ts     # Tauri event listeners
│   │   └── useGlobalShortcut.ts  # Keyboard shortcuts
│   ├── store/                    # Zustand state management
│   │   └── voiceStore.ts         # Global app state
│   └── page.tsx                  # Main page
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── lib.rs                # App setup + command registration
│   │   ├── audio.rs              # Audio capture + VAD + export
│   │   ├── transcription.rs      # Deepgram streaming
│   │   ├── secrets.rs            # OS keychain storage
│   │   ├── tts.rs                # Native text-to-speech
│   │   └── agents/               # AI agent implementations
│   │       ├── action_items.rs   # GPT-4o task extraction
│   │       ├── tone_shifter.rs   # Claude tone rewriting
│   │       ├── translator.rs     # GPT-4o translation
│   │       └── music_matcher.rs  # Music search + mood
│   ├── capabilities/             # Tauri permissions
│   └── tauri.conf.json           # Tauri configuration
└── __tests__/                    # Unit tests
```

## Development Commands

```bash
# Frontend development
pnpm dev              # Start Next.js dev server
pnpm build            # Build static export

# Tauri desktop app
pnpm tauri dev        # Run with hot reload
pnpm tauri build      # Build production binary

# Testing
pnpm test             # Run Vitest unit tests
pnpm test:e2e         # Run Playwright E2E tests

# Rust backend
cd src-tauri
cargo check           # Type check
cargo clippy          # Lint
cargo build           # Build
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+Shift+V` | Toggle window visibility |
| `Escape` | Hide window |

## Window Configuration

- **Default size**: 1000×700
- **Min size**: 700×500
- **Max size**: 1500×1200
- **Style**: Frameless, transparent, always-on-top

## Supported Languages (Translation)

- Auto-detect (source only)
- English, German, Spanish, French, Italian
- Portuguese, Dutch, Russian
- Japanese, Chinese, Korean, Arabic

## Tone Presets

| Tone | Description | Use Cases |
|------|-------------|-----------|
| Professional | Business-appropriate | Work emails, Reports |
| Casual | Relaxed and informal | Messages, Social |
| Friendly | Warm and approachable | Support, Welcome |
| Formal | Official and structured | Legal, Academic |
| Empathetic | Understanding | Support, Apologies |
| Assertive | Confident and direct | Negotiations, Leadership |
| Diplomatic | Tactful and balanced | Feedback, Conflicts |
| Enthusiastic | Energetic and positive | Marketing, Announcements |

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `pnpm test`
5. Submit a pull request
