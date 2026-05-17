## Athena — 2026-05-17

**Scanned:**
- `app/layout.tsx`, `app/page.tsx`, `app/settings/page.tsx`, `app/components/*.tsx`
- `app/hooks/useTauriEvents.ts`, `app/hooks/useGlobalShortcut.ts`, `app/hooks/useDeepgramStreaming.ts`
- `app/store/voiceStore.ts`
- `src-tauri/src/lib.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/transcription.rs`, `src-tauri/src/secrets.rs`
- `src-tauri/src/platform/{mod,audio/mod,audio/desktop,audio/mobile,secrets/mod,secrets/apple}.rs`
- `src-tauri/src/agents/{action_items,tone_shifter,music_matcher,translator,dev_log,brain_dump,mental_mirror}.rs`

**Findings:** 11

---

### F1: No React Error Boundary anywhere in the application tree

- **Severity (proposed):** High
- **Location:** `app/layout.tsx` (root) and `app/page.tsx` (Home) — architectural; no `<ErrorBoundary>` component exists in the codebase (`grep -r "ErrorBoundary"` returns nothing).
- **Crash-Mode:** UI-Freeze
- **Trigger:** Any uncaught render-time exception in `<TranscriptDisplay>`, `<AgentResults>`, `<SyncPairing>`, etc. — e.g. a malformed `mentalMirrorResult` from a partial SSE stream, or a `null` field where the type says non-null — propagates to the React root and unmounts the entire app. Tauri's webview shows a white screen with no recovery path; the user must Cmd+Shift+V to hide/reshow, which does not remount React.
- **Repro-Snippet:** Backend emits a `mental-mirror-complete` event whose payload is missing the `disclaimer` field (or with an unexpected discriminator on `BrainDumpTask.quadrant`); the consuming component reads a deep property and throws. App goes blank.
- **Fix-Sketch:** Introduce a top-level `<ErrorBoundary>` in `app/layout.tsx` with a user-visible fallback ("Something went wrong — Reset") that calls `voiceStore.reset()` and a per-feature boundary around each agent-result component so one bad payload cannot kill the whole UI. Boundary should log the error to a Rust-side `report_frontend_error` command for telemetry.
- **iOS-Relevanz:** Same-Pfad-iOS — even more critical on iOS where users cannot easily reopen a frozen WebView.
- **Confidence:** High

---

### F2: Deepgram WebSocket has no reconnect / backoff strategy

- **Severity (proposed):** Critical
- **Location:** `src-tauri/src/transcription.rs:131-173` (read task) and `175-189` (write task)
- **Crash-Mode:** Silent-Loss
- **Trigger:** Any transient network event — Wi-Fi handoff, VPN reconnect, Deepgram-side timeout, mobile cell-to-Wi-Fi switch — causes the receive loop to hit `Ok(Message::Close(...))` or `Err(...)` and `break`. The cleanup sets `is_streaming = false`, but the audio capture thread keeps running and silently drops samples (the `try_send` on a now-disconnected channel succeeds against a dead Deepgram socket because the sender end is dropped only on `stop_deepgram_stream`). The user sees the mic visualizer moving and "recording" UI, but no transcripts arrive.
- **Repro-Snippet:** Start recording, disable Wi-Fi for 5 s, re-enable. Speak. No transcripts appear; UI never indicates the connection died.
- **Fix-Sketch:** Wrap the connect+read loop in a supervised actor with exponential backoff (250 ms → 8 s, max 5 attempts), emit a `deepgram-disconnected`/`deepgram-reconnecting` event so the frontend can show a banner, and tear down audio capture if reconnect exhausts. On iOS this must additionally check for `AVAudioSession` and `NWPathMonitor` cues before retrying.
- **iOS-Relevanz:** Same-Pfad-iOS — network transitions are the norm on mobile, not the exception.
- **Confidence:** High

---

### F3: Audio capture thread has no supervisor; stream errors silently log and continue

- **Severity (proposed):** High
- **Location:** `src-tauri/src/audio.rs:191-194` (CPAL error callback) and `src-tauri/src/audio.rs:72-82` (spawn-and-forget)
- **Crash-Mode:** Silent-Loss
- **Trigger:** CPAL's stream error callback (`|err| tracing::error!(...)`) only logs — it does not flip `IS_RECORDING` to false, does not emit `recording-error`, and does not attempt restart. If the audio device is unplugged, the OS revokes mic access mid-session, or the stream underruns past recovery, the recording UI stays "active" while no samples are produced.
- **Repro-Snippet:** Start recording with a USB mic. Unplug the mic. UI remains in `recording` state; transcript stops growing; no error is shown.
- **Fix-Sketch:** Convert audio capture into a supervised task with a watchdog: the error callback should signal a `tokio::sync::Notify` that triggers either (a) a restart with the new default device or (b) a clean teardown with a user-facing `recording-error` event. The 100 ms sleep loop at `audio.rs:203-205` should also check a "stream-healthy" flag, not just `IS_RECORDING`.
- **iOS-Relevanz:** Same-Pfad-iOS — see F6, F8, F9. AVAudioSession interruptions surface here as stream errors.
- **Confidence:** High

---

### F4: voiceStore state can be desynced from real recording state

- **Severity (proposed):** Medium
- **Location:** `app/store/voiceStore.ts:99,254` (recordingState) and the interaction with `audio.rs` global `IS_RECORDING`
- **Crash-Mode:** Race
- **Trigger:** The store's `recordingState` is driven by Tauri events (`recording-started`, `recording-stopped`), but the Rust side has its own `IS_RECORDING: AtomicBool` and the Deepgram side has its own `is_streaming: bool`. There is no single source of truth and no reconciliation on app focus / window show. After F2 or F3 fires, the Rust state can be `is_streaming=false` while the store still reads `recordingState='recording'`. The user clicks Stop → `stop_recording` returns `Err("Not recording")` → toast error shows but UI stays stuck.
- **Repro-Snippet:** Trigger F2 (network blip). Click Stop → see "Not recording" error toast. The Record button is now in an unrecoverable state until a hard refresh.
- **Fix-Sketch:** Introduce a periodic state reconciliation on window-focus (`invoke('is_recording')` + `invoke('is_deepgram_streaming')`) that overwrites the store, and have `stop_recording` be idempotent (return `Ok(())` if not recording). Treat the Rust side as the authoritative state machine; the store is a projection.
- **iOS-Relevanz:** Same-Pfad-iOS — desync is much more likely after background suspension.
- **Confidence:** High

---

### F5: AssemblyAI poll loop and all LLM HTTP clients have no client-side timeout

- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/transcription.rs:238` (`reqwest::Client::new()`), and every agent in `src-tauri/src/agents/*.rs:64,80,91,96,136,144,159,164,172,180,211,218,248`
- **Crash-Mode:** UI-Freeze
- **Trigger:** All `reqwest::Client::new()` calls use defaults — no `connect_timeout`, no `timeout`. A hung Anthropic/OpenAI/AssemblyAI SSE stream or stalled TCP connection leaves the streaming task and the frontend "Processing..." indicator forever. The user has no cancel button; reset only clears UI state but the orphaned task continues to consume the API key quota.
- **Repro-Snippet:** Block egress to `api.anthropic.com` mid-stream after the first chunk. The "Shifting tone..." spinner never stops; `tone-shift-complete` is never emitted.
- **Fix-Sketch:** Build one shared `reqwest::Client` with `connect_timeout(5s)`, `timeout(120s)` (or `read_timeout` for streaming), wrap streaming reads in `tokio::time::timeout(Duration::from_secs(30))` per chunk, and add a per-agent cancellation handle stored in a `tokio::sync::CancellationToken` map so the frontend can call `cancel_agent`.
- **iOS-Relevanz:** Same-Pfad-iOS — background suspension makes hung in-flight requests near-certain.
- **Confidence:** High

---

### F6: No concept of pause/resume for AVAudioSession interruptions (iOS)

- **Severity (proposed):** Critical
- **Location:** Architectural — there is no iOS lifecycle bridge anywhere. `grep AVAudioSession` returns nothing in `src-tauri/`. `src-tauri/src/platform/audio/mobile.rs` is a `NotSupported` stub.
- **Crash-Mode:** Silent-Loss
- **Trigger:** On iOS, an incoming phone call, Siri invocation, system alarm, or another app activating its own AVAudioSession will interrupt or duck the app's audio session. Without registering for `AVAudioSession.interruptionNotification` and reacting to `.began` / `.ended`, the mic stream silently stops producing samples and Aurus never recovers — even after the interruption ends.
- **Repro-Snippet:** (Future iOS build) Start recording. Receive a call. Decline. App resumes foreground but transcript no longer grows.
- **Fix-Sketch:** Define a `PausableAudioPipeline` trait at the platform layer with explicit `pause()` / `resume()` semantics. The iOS implementation registers for `AVAudioSession.interruptionNotification` via a Swift bridge or `objc2` crate and emits `audio-pipeline-paused` / `audio-pipeline-resumed` Tauri events. The frontend shows an "Interrupted — tap to resume" overlay rather than silently failing. Deepgram WebSocket should also be paused (or torn down + reconnected on resume; see F2).
- **iOS-Relevanz:** iOS-Only
- **Confidence:** High

---

### F7: No background-suspension strategy — Deepgram WebSocket and LLM requests die invisibly on iOS

- **Severity (proposed):** Critical
- **Location:** Architectural — no `applicationDidEnterBackground` / `applicationWillResignActive` hook is registered; no Tauri Mobile lifecycle plugin is used.
- **Crash-Mode:** Silent-Loss
- **Trigger:** When iOS suspends the app (home button, app switcher, screen lock without background-audio entitlement), all open sockets are closed by the OS after ~30 s, in-flight `reqwest` tasks are paused, and the tokio runtime is frozen. On resume, the Deepgram WS is a half-dead connection, the Tauri event listeners are still attached, and the store says `recordingState='recording'`. Resulting transcript is silently truncated.
- **Repro-Snippet:** (Future iOS build) Start recording. Lock screen for 60 s. Unlock. App appears resumed but no transcripts; no error.
- **Fix-Sketch:** On `applicationWillResignActive`: emit `app-backgrounded`, gracefully `stop_deepgram_stream`, persist current transcript buffer to disk, mark store as `paused`. On `applicationDidBecomeActive`: validate all stateful resources (mic permission, WS, secret access) and re-establish or surface an explicit "Session interrupted — resume?" UI. Apply for the `audio` background mode only if continuous capture is a product requirement, because that mode comes with App Store review friction.
- **iOS-Relevanz:** iOS-Only
- **Confidence:** High

---

### F8: Unbounded growth of recording buffer and transcript — OOM risk on iOS

- **Severity (proposed):** High
- **Location:** `src-tauri/src/audio.rs:13` (`RECORDING_BUFFER: Lazy<Mutex<Vec<i16>>>`) and `app/store/voiceStore.ts:264-273` (`appendTranscript` concatenation with no cap)
- **Crash-Mode:** Resource-Leak
- **Trigger:** `RECORDING_BUFFER` grows by 32 KB/s of i16 PCM at 16 kHz mono for the entire recording with no ceiling — 1 hour ≈ 115 MB resident. `transcript: string` in Zustand grows by every final segment from Deepgram with no rotation. On iOS, the OS issues `didReceiveMemoryWarning` long before desktop would notice, and the app gets jetsam-killed without warning. On desktop a 4-hour brainstorming session pushes ~460 MB into a `Vec<i16>` lock that blocks every audio callback.
- **Repro-Snippet:** Start recording. Leave running for ~30 min. Observe RES memory in Activity Monitor climbing linearly. On a device with 4 GB RAM (older iPad), expect jetsam kill within an hour.
- **Fix-Sketch:** (a) Cap `RECORDING_BUFFER` to a rolling N-minute window or stream to disk (WAV append) after every ~1 minute, freeing memory. (b) Cap `transcript` length in the store with a "transcript truncated — see file" affordance, or move history to an out-of-band IndexedDB/file-backed store. (c) Register an iOS memory-warning handler in the Tauri Mobile entrypoint that aggressively flushes both buffers.
- **iOS-Relevanz:** Same-Pfad-iOS (problem exists on desktop; severity is much higher on iOS).
- **Confidence:** High

---

### F9: Mic permission revocation at runtime has no detection path

- **Severity (proposed):** High
- **Location:** Architectural — no permission state machine. `audio.rs:84-110` reads `default_input_device()` once at start; no re-check, no permission-changed event.
- **Crash-Mode:** Silent-Loss
- **Trigger:** macOS, Windows, and iOS all allow users to revoke microphone permission while the app runs. On macOS, CPAL will start receiving zero-filled buffers (or stream-error); on iOS, the `AVAudioSession` will fail to activate and samples stop. Aurus has no `permission-revoked` event and no recurring check — the UI stays in `recording` state with a flat waveform.
- **Repro-Snippet:** Start recording on macOS. Open System Settings → Privacy → Microphone → revoke Aurus. UI keeps spinning; no error.
- **Fix-Sketch:** Introduce a permission probe at audio-start (and on app-foreground for iOS) that calls the platform API (`AVCaptureDevice.authorizationStatus(for: .audio)` on Apple, `MediaDevices.permissions.query()` fallback on web). Emit `mic-permission-changed` from a watcher. On iOS, also re-probe after `applicationDidBecomeActive` (see F7).
- **iOS-Relevanz:** Same-Pfad-iOS — iOS Settings is the primary revocation vector.
- **Confidence:** High

---

### F10: Tauri event listeners in `useTauriEvents` over-register if effect re-runs

- **Severity (proposed):** Medium
- **Location:** `app/hooks/useTauriEvents.ts:161-447`
- **Crash-Mode:** Resource-Leak
- **Trigger:** The effect's dependency array (lines 421-447) contains ~25 Zustand setters. Although the setters are stable references from `create()`, the destructured selector pattern in `useVoiceStore()` returns a new object each render. If the parent (`page.tsx`) re-renders and a dev ever switches the selector to a non-shallow form, the effect tears down and reruns. More importantly, `listen()` is async — between the `await import(...)` and pushing the unlisten handle into `listeners`, an unmount can fire its cleanup before any handles are registered, leaking dozens of listeners across hot-reloads and unmount races.
- **Repro-Snippet:** Hot-reload `useTauriEvents.ts` 10x during development → Rust-side event subscriber count keeps growing (visible if you log `Emitter` listener count). In production: navigate between `/` and `/settings` 50x; each navigation can leak one async-race listener.
- **Fix-Sketch:** Use a single AbortController + an `isCancelled` flag captured before each `await listen(...)`; if cancelled after the await, immediately call `unlisten()`. Better: extract event subscriptions into a single async helper that returns a composite unlisten, awaited in the effect via a `Promise<() => void>`. Move the listeners into a module-scoped singleton initialized once at app mount (the events are app-global, not page-scoped).
- **iOS-Relevanz:** Same-Pfad-iOS — leak rate is higher because lifecycle resume re-runs effects.
- **Confidence:** Medium

---

### F11: `transcribe_local_whisper` is unconditionally cfg-gated off on iOS with no documented mobile transcription fallback

- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/transcription.rs:337-469` (all whisper-rs code is `#[cfg(not(any(target_os = "ios", target_os = "android")))]`) and `lib.rs:150-151` (command not registered on mobile)
- **Crash-Mode:** Silent-Loss
- **Trigger:** Architecturally, iOS has *only* Deepgram (online) and AssemblyAI (online) — no offline path. If the user is offline on iOS, the recording UI works (samples are captured by Web Audio), but every Deepgram WS attempt fails, every AssemblyAI POST fails, and there is no on-device transcription fallback. The UI currently has no concept of "offline mode" and no degraded experience — the user sees a recording with no transcript.
- **Repro-Snippet:** (Future iOS build) Airplane mode → start recording → speak → no transcript ever appears.
- **Fix-Sketch:** Either (a) port `whisper.cpp` to iOS via `whisper-rs` with Metal acceleration (the upstream supports it; the cfg-gate is conservative, not technical), or (b) explicitly degrade to "Offline — transcript unavailable" UI state, persist the audio file, and offer a "Transcribe when online" action that runs on reconnect. Decision should be recorded in `docs/ARCHITECTURE.md`.
- **iOS-Relevanz:** iOS-Only
- **Confidence:** Medium
