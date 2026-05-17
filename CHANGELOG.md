# Changelog

All notable changes to Aurus Voice Intelligence will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-01-29

### Added

#### AI Agents
- **Action Items** - Extract tasks, commitments, and deadlines from voice transcripts using GPT-4o
- **Tone Shifter** - Rewrite text in 8 different tones (Professional, Casual, Friendly, Formal, Empathetic, Assertive, Diplomatic, Enthusiastic) with intensity control using Claude Sonnet 4
- **Translator** - Translate to 12+ languages with automatic source language detection using GPT-4o
- **Dev-Log** - Generate conventional commit messages, Jira/Linear tickets, and Slack updates from developer rambling using GPT-4o
- **Brain Dump** - Categorize unstructured thoughts into Eisenhower Matrix tasks, creative ideas, and notes using GPT-4o
- **Mental Mirror** - Transform daily reflections into compassionate "Letter to My Future Self" with psychological frameworks using GPT-4o

#### Voice Processing
- Real-time streaming transcription via Deepgram Nova-2
- Voice Activity Detection (VAD) with visual waveform feedback
- Audio recording export to WAV format
- Support for German language transcription
- Fallback transcription via AssemblyAI and local Whisper

#### User Interface
- Spotlight-style frameless window interface
- Global hotkey activation (Cmd+Shift+V)
- Resizable window with min/max constraints
- Real-time transcript display with interim results
- Streaming AI responses with blur effect during generation
- Copy to clipboard for all outputs
- Native Text-to-Speech for reading results aloud
- Translation of any agent output to 12+ languages

#### Platform Features
- macOS support (Apple Silicon + Intel)
- Windows support
- Linux support (Ubuntu/Debian)
- Secure API key storage in OS keychain
- Native system TTS integration

#### Developer Experience
- Comprehensive documentation with Mermaid diagrams
- GitHub Actions CI/CD pipeline for multi-platform releases
- Session checkpoint system (.memory/) for context persistence
- 33 passing unit tests

### Technical Stack
- **Frontend:** Next.js 14 (static export), React 18, Zustand, TailwindCSS
- **Backend:** Rust, Tauri v2, CPAL (audio), tokio (async)
- **AI Services:** OpenAI GPT-4o, Anthropic Claude Sonnet 4, Deepgram Nova-2

---

## [1.2.0] - 2026-03-18

### Fixed
- Prevent window hiding when global shortcut unavailable
- CSP — add Tauri IPC protocol to connect-src
- Security audit remediation — 32 findings across 5 severity phases

### Added
- iOS build support — gate datachannel behind desktop cfg

---

## [Unreleased]

### Security & Stability — Crash-Stability Audit 2026-05-17

5 parallel persona-lens agents (Nemesis, Aletheia, Ipcha, Athena, Metis) audited the full stack with iOS lookahead. Cael judge consolidated 57 raw findings into 43 canonical entries (7 Critical, 11 High, 18 Medium, 7 Low). 16 in-session fixes shipped; 2 iOS-only Criticals + 24 Medium/Low findings tracked as nyxCore Action Points for Sub-Project A.

#### Critical fixes
- **C3** Panic-safe audio capture with RAII guard — IS_RECORDING resets on panic-unwind, never stuck "already recording"
- **C4** First-run API-key gate banner with focus-probe re-check
- **C5** Classified HTTP errors (401/403, 402, 429, 408/504, 5xx) replace one-size-fits-all "Service temporarily unavailable"
- **C6** Mic permission watchdog — emits `mic-permission-denied` after 3 s of no callbacks
- **C7** Deepgram WS reconnect with exponential backoff (250 ms → 8 s, max 5 attempts)

#### High fixes
- **H1** WebRTC `DcShared` mutex poison-tolerant (`unwrap_or_else(|p| p.into_inner())`) — prevents SIGABRT across FFI
- **H2** SPAKE2 listener loop with attempt budget — one hostile probe no longer burns pairing session
- **H3** Audio capture supervisor — stream-error callback now breaks the loop and emits `recording-error`
- **H4** Consecutive audio-drop counter → `transcription-degraded` event after 3 consecutive drops
- **H5** Hotkey hide / Escape both stop recording — no more silent battery/quota drain
- **H6** `useTauriEvents` + `useAudioForwarding` hoisted to root layout — surviving navigation
- **H7** Rolling-window cap on `RECORDING_BUFFER` (30 min / 57 MiB) + 50k-char transcript cap
- **H8** Agents reject null/missing API content instead of silent empty-string success
- **H9** Top-level React `<ErrorBoundary>` — no more white-screen on render exceptions
- **H10** Deepgram WS frame cap at 64 KiB — MITM JSON-bombs now Capacity-error cleanly
- **H11** 20 MiB CRDT state budget per peer — crafted yrs updates can't grow unbounded

#### Low fixes
- **L1** `calculate_energy` filters non-finite floats and clamps to [-1, 1] before squaring

#### Test infrastructure
- Added `proptest` as dev-dependency
- 8 property tests for `audio.rs` (resampler/VAD/mono never panic on adversarial input)
- 4 negative tests for Deepgram frame parser (JSON-bomb, truncated, wrong-shape)
- 33+ regression unit tests covering every Critical+High fix
- All 97 Rust tests green; clippy clean with `-D warnings`

#### Deferred to Sub-Project A (iOS build)
- **C1** AVAudioSession interruption handling
- **C2** iOS background-suspension lifecycle hooks

### Planned
- Music Matcher agent (Q-Records API integration)
- Real email delivery for Mental Mirror
- Multi-language transcription
- Conversation history with search
- Custom agent builder

[1.2.0]: https://github.com/mrwind-up-bird/ai-voiceagent/releases/tag/v1.2.0
[1.0.0]: https://github.com/mrwind-up-bird/ai-voiceagent/releases/tag/v1.0.0
[Unreleased]: https://github.com/mrwind-up-bird/ai-voiceagent/compare/v1.2.0...HEAD
