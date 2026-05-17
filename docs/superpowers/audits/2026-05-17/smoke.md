# End-of-Session Smoke Checklist — 2026-05-17

**Status:** Deferred for user. This audit session ran autonomously and cannot perform live UI interaction. Run `pnpm tauri dev` after pulling the new commits and verify the boxes below.

## Pre-flight

- [ ] Working tree clean? `git status` should show only `.claude/settings.local.json` and `.memory/` as modified/untracked.
- [ ] `cargo test` — expect 97 green
- [ ] `cargo clippy --tests -- -D warnings` — clean
- [ ] `pnpm tsc --noEmit` — only pre-existing test-file errors (the `agents.test.ts` `unknown` type issues are pre-existing)

## Cold-start

- [ ] `pnpm tauri dev` launches without panic
- [ ] App opens at Cmd+Shift+V (or fallback Cmd+Shift+A)
- [ ] **C4 first-run gate**: with all API keys removed (or first install), amber banner "No transcription API key configured" appears with "Open settings" link

## Settings flow

- [ ] Open Settings via the cog icon
- [ ] Add a valid Deepgram key → save
- [ ] Return to main → banner gone
- [ ] Re-open Settings, delete the key, return → banner reappears (focus-probe)

## Recording — happy path

- [ ] Click mic → button turns red, no error toast
- [ ] Within ~3 s of speaking, transcript appears (Deepgram nova-2 German)
- [ ] Click mic again → button green, transcript final

## Recording — error UX

- [ ] **C5**: with a deliberately invalid Deepgram key, click mic. Expect "Authentication failed. Please check your API key in Settings." (NOT the old "Service temporarily unavailable")
- [ ] **C6**: in macOS System Settings → Privacy → Microphone → toggle Aurus OFF. Click record. Within 3 s expect "No audio detected. Check microphone permission in System Settings → Privacy."

## Recording — supervisor

- [ ] **H3**: start recording with a USB mic, unplug it mid-session. Expect `recording-error` event surfaced as a toast within ~100 ms (no UI freeze).
- [ ] **H7-rust**: long-session smoke (skipped unless you have 30+ min) — record continuously, observe `RECORDING_BUFFER` does not grow past ~57 MB resident.

## Hotkey hygiene

- [ ] **H5**: start recording, press Cmd+Shift+V (hide window). Re-show with Cmd+Shift+V. Verify the mic returned to idle (recording auto-stopped on hide).
- [ ] **H5 (Escape)**: start recording, press Escape (with global shortcut registered). Re-show via hotkey. Verify recording stopped.

## Navigation

- [ ] **H6**: start recording, navigate to Settings mid-sentence, return. Verify any words spoken during the transit are NOT lost (useTauriEvents lives at layout level, listeners survive route changes).

## Network resilience

- [ ] **C7 reconnect**: start recording with stable Wi-Fi. Toggle Wi-Fi off for 5 s, then on. Expect:
  - `deepgram-disconnected` log/console line
  - Frontend retries `start_deepgram_stream` with 250 ms → 500 → 1 s → 2 s → 4 s backoff
  - Within ~30 s of reconnect, transcripts resume
  - If reconnect exhausts (5 attempts), error toast surfaces

## React resilience

- [ ] **H9 Error Boundary**: deliberately throw in `<AgentResults>` (e.g. via dev tools) — verify "Something went wrong — Reset view" appears with a working reset button, app does not white-screen.

## Multi-agent flow

- [ ] After a recording, click each of: Action Items, Tone Shift, Music Match, Translate, Brain Dump, Dev Log, Mental Mirror
- [ ] Each agent either succeeds OR shows the appropriate classified error (`401` → "Authentication failed", `429` → "Rate limit hit", etc., per C5)
- [ ] **H8**: tone-shift / translate with a deliberately invalid Anthropic/OpenAI key. Expect Err, NOT a silent empty-string result.

## Shutdown

- [ ] Cmd+Q while idle → app exits cleanly
- [ ] Cmd+Q while recording → expect the in-memory buffer is lost (M12, deferred), but no panic and no zombie process

## iOS lookahead (mental walk-through only — no iOS target yet)

- [ ] **C1 (deferred to Sub-Project A)**: when iOS target lands, register `AVAudioSession.interruptionNotification` and pause/resume the pipeline.
- [ ] **C2 (deferred to Sub-Project A)**: implement `applicationWillResignActive` → persist transcript, gracefully stop Deepgram WS.

## Sign-off

When all green boxes are checked: commit `smoke.md` with check-marks, then tag the audit complete:

```bash
git add docs/superpowers/audits/2026-05-17/smoke.md
git commit -m "audit(crash): smoke checklist signed off — 2026-05-17"
```
