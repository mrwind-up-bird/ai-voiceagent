# Operations Runbook

## Deployment Procedures

### Building for Production

```bash
# Build Next.js static export
pnpm build

# Build Tauri application
pnpm tauri build
```

Output locations:
- Next.js: `./out/`
- macOS: `src-tauri/target/release/bundle/dmg/`
- Windows: `src-tauri/target/release/bundle/msi/`

### Release Checklist

1. Update version in `package.json` and `src-tauri/tauri.conf.json`
2. Run full test suite: `pnpm test && pnpm test:e2e`
3. Run Rust checks: `cd src-tauri && cargo clippy && cargo test`
4. Build production binary: `pnpm tauri build`
5. Test the built application manually
6. Create release notes
7. Tag release in git

## Monitoring and Logs

### Log Locations

- **Development**: Console output from `pnpm tauri dev`
- **Production (macOS)**: `~/Library/Logs/Voice Intelligence Hub/`
- **Rust logs**: Controlled by `RUST_LOG` environment variable

### Enable Debug Logging

```bash
RUST_LOG=debug pnpm tauri dev
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

## Common Issues and Fixes

### Microphone Not Working

**Symptoms**: No audio captured, VAD shows no energy

**Solutions**:
1. Check macOS permissions: System Settings > Privacy & Security > Microphone
2. Verify microphone works in QuickTime or another app
3. Restart the application after granting permission

### Transcription Returns Empty

**Symptoms**: Recording works but no transcript appears

**Solutions**:
1. Verify Deepgram API key is configured in Settings
2. Check network connectivity
3. Look for errors in console output
4. Verify the key has sufficient credits

### Window Not Responding

**Symptoms**: UI freezes, clicks don't register

**Solutions**:
1. Check if a long-running operation is blocking (transcription, API call)
2. Force quit and restart: `pkill -f "Voice Intelligence Hub"`
3. Clear app data and restart

### Global Shortcut Not Working

**Symptoms**: Cmd+Shift+V doesn't toggle window

**Known Issue**: macOS may block the shortcut if another app has it registered.

**Workarounds**:
1. Click the app icon in the dock
2. Use a different shortcut (requires code change)
3. Grant Accessibility permissions in System Settings

### Build Failures

**Rust compilation errors**:
```bash
cd src-tauri
cargo clean
cargo build
```

**Next.js build errors**:
```bash
rm -rf .next out node_modules
pnpm install
pnpm build
```

## Rollback Procedures

### Quick Rollback

1. Stop the running application
2. Install the previous version from backup/release
3. Verify functionality

### Data Preservation

API keys are stored in OS keychain and persist across app versions. No rollback needed for credentials.

### Emergency Contacts

- Repository: Check issues for known problems
- Logs: Always collect logs before reporting issues

## Health Checks

### Manual Verification

1. **Audio Capture**: Click record, speak, verify waveform animation
2. **Transcription**: Speak clearly, verify text appears
3. **Agents**: Select each agent, verify results appear
4. **Settings**: Add/remove API key, verify persistence

### Automated Checks

```bash
# Run unit tests
pnpm test --run

# Run E2E tests
pnpm test:e2e

# Rust checks
cd src-tauri && cargo check && cargo clippy
```

## Performance Notes

- Binary size target: < 10MB (before models)
- Voice-to-transcript latency: < 500ms (cloud mode)
- Memory usage: Monitor for leaks during long sessions
- Audio buffer: 100ms chunks at 16kHz (1600 samples)
