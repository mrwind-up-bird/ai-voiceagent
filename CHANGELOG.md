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

## [Unreleased]

### Planned
- Music Matcher agent (Q-Records API integration)
- Real email delivery for Mental Mirror
- Multi-language transcription
- Conversation history with search
- Custom agent builder

[1.0.0]: https://github.com/mrwind-up-bird/ai-voiceagent/releases/tag/v1.0.0
[Unreleased]: https://github.com/mrwind-up-bird/ai-voiceagent/compare/v1.0.0...HEAD
