# Contributing Guide

## Development Workflow

### Prerequisites

- **Node.js** 18+ with pnpm
- **Rust** toolchain (rustup, cargo)
- **Tauri CLI** (`pnpm tauri`)
- **macOS**: Xcode Command Line Tools
- **Windows**: Visual Studio Build Tools

### Quick Start

```bash
# Install dependencies
pnpm install

# Start development (Tauri + Next.js)
pnpm tauri dev
```

## Available Scripts

| Script | Command | Description |
|--------|---------|-------------|
| `dev` | `pnpm dev` | Start Next.js development server on localhost:3000 |
| `build` | `pnpm build` | Build Next.js static export to `./out` directory |
| `start` | `pnpm start` | Start Next.js production server (not used with Tauri) |
| `lint` | `pnpm lint` | Run ESLint on the codebase |
| `test` | `pnpm test` | Run Vitest unit tests |
| `test:e2e` | `pnpm test:e2e` | Run Playwright end-to-end tests |
| `tauri` | `pnpm tauri` | Run Tauri CLI commands |

### Tauri Commands

```bash
pnpm tauri dev      # Development with hot reload
pnpm tauri build    # Production binary build
```

### Rust Commands

```bash
cd src-tauri
cargo build         # Build Rust backend
cargo check         # Type check without building
cargo clippy        # Lint Rust code
cargo test          # Run Rust unit tests
```

## Environment Setup

### API Keys

API keys are stored securely in the OS keychain (macOS Keychain / Windows Credential Manager). No `.env` file is used.

Configure keys via the Settings page in the app:

| Key | Service | Purpose |
|-----|---------|---------|
| `deepgram` | Deepgram | Real-time speech-to-text (Nova-2) |
| `assembly_ai` | AssemblyAI | Fallback transcription service |
| `openai` | OpenAI | GPT-4o for action item extraction |
| `anthropic` | Anthropic | Claude for tone shifting |
| `qrecords` | Q-Records | Music matching service |

### macOS Microphone Permission

The app requires microphone access. On first run, macOS will prompt for permission. If denied, enable in:

**System Settings > Privacy & Security > Microphone > Voice Intelligence Hub**

## Testing Procedures

### Unit Tests

```bash
# Run all tests
pnpm test

# Watch mode
pnpm test -- --watch

# Single file
pnpm test -- agents.test.ts

# With coverage
pnpm test -- --coverage
```

Tests are located in `__tests__/` and use Vitest with React Testing Library.

### E2E Tests

```bash
# Run Playwright tests
pnpm test:e2e

# With UI
pnpm test:e2e -- --ui

# Specific test
pnpm test:e2e -- voice-flow.spec.ts
```

E2E tests are in `e2e/` and run against the Next.js dev server.

### Rust Tests

```bash
cd src-tauri
cargo test
```

## Code Style

- **TypeScript**: Strict mode enabled
- **Rust**: Follow `cargo clippy` recommendations
- **Formatting**: Prettier (JS/TS), rustfmt (Rust)

## Architecture Notes

- Next.js uses `output: 'export'` (static files only)
- All API calls happen in Rust, never in frontend
- Audio is captured at native rate, resampled to 16kHz in Rust
- Audio goes directly from Rust to Deepgram (no frontend round-trip)
