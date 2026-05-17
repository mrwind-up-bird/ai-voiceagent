## Metis — 2026-05-17
**Scanned:** `src-tauri/src/lib.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/transcription.rs`, `src-tauri/src/secrets.rs`, `src-tauri/src/agents/*.rs`, `src-tauri/tauri.conf.json`, `src-tauri/Info.plist`, `app/page.tsx`, `app/settings/page.tsx`, `app/components/VoiceInput.tsx`, `app/components/TranscriptDisplay.tsx`, `app/hooks/useTauriEvents.ts`, `app/hooks/useGlobalShortcut.ts`, `app/hooks/useDeepgramStreaming.ts`, `app/store/voiceStore.ts`
**Findings:** 11

---

### F1: First-run with zero API keys looks identical to a working app — user records into a void
- **Severity (proposed):** Critical
- **Location:** `app/page.tsx:22-219`, `app/components/VoiceInput.tsx:88-110`, `src-tauri/src/secrets.rs:89` (`has_api_keys` exists but is **never called from the frontend**)
- **Crash-Mode:** Silent-Loss
- **Trigger:** User installs the app, opens it, hits the record button before visiting Settings.
- **Repro-Snippet:** Fresh install → click big mic button → talk → nothing happens → mic stays red.
- **Fix-Sketch:** On `app/page.tsx` mount, invoke `has_api_keys`. If false, route to `/settings` with a banner ("Add a Deepgram or AssemblyAI key to start"). Disable the record button until at least one transcription provider is configured, with a tooltip explaining why.
- **iOS-Relevanz:** Same-Pfad-iOS — iOS users hit this even harder since they have no `start_recording` Rust fallback and rely on web audio + Deepgram.
- **Confidence:** High

---

### F2: Wrong/expired API keys produce no actionable error — both Deepgram and all agents fail silently or with generic "Service temporarily unavailable"
- **Severity (proposed):** Critical
- **Location:** `src-tauri/src/agents/action_items.rs:88-98`, `src-tauri/src/agents/tone_shifter.rs:166-172,242-248`, `src-tauri/src/agents/brain_dump.rs:205-213`, `src-tauri/src/agents/dev_log.rs:121-129`, `src-tauri/src/agents/translator.rs:170-176`, `src-tauri/src/agents/mental_mirror.rs:116-124`, `src-tauri/src/agents/music_matcher.rs:104-211`, `src-tauri/src/transcription.rs:109-117`, `app/components/VoiceInput.tsx:104-106`
- **Crash-Mode:** Silent-Loss
- **Trigger:** User pastes a typo'd or expired API key in Settings, returns to main UI, tries any feature.
- **Repro-Snippet:** Settings → paste "sk-xxxxx" → Save → back to main → record → stop → click action items → see "Service temporarily unavailable. Please try again." forever. User cannot distinguish bad key from rate limit from outage.
- **Fix-Sketch:** Distinguish HTTP 401/403 (auth) from 429 (rate limit) from 5xx (outage) and surface a different message per case. For Deepgram WS, parse the close frame reason (1008/4xxx) and emit a `deepgram-auth-failed` event the UI can react to. In `VoiceInput.tsx`, when `start_deepgram_stream` rejects, do NOT call `start_recording` — instead show a red toast with "Check your Deepgram key in Settings" and a deep-link.
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** High

---

### F3: Microphone permission denied at OS level → app shows red recording button with no error
- **Severity (proposed):** Critical
- **Location:** `src-tauri/src/audio.rs:59-82` (thread spawn) and `:84-88` (`default_input_device`)
- **Crash-Mode:** Silent-Loss / UI-Freeze
- **Trigger:** macOS user with mic disabled in System Settings → Privacy & Security → Microphone, or first-launch tap before the OS prompt is granted.
- **Repro-Snippet:** Settings.app → Privacy → Microphone → toggle Aurus OFF → click record. `setRecordingState('recording')` runs immediately (`VoiceInput.tsx:93`), the button turns red and pulses, but CPAL silently delivers zero callbacks. No `recording-error` is emitted because `default_input_device()` still returns Some on macOS even when permission is denied — CPAL just gets a silent stream.
- **Fix-Sketch:** Pre-flight the permission via `AVAudioApplication.requestRecordPermission` (Tauri macOS plugin or thin FFI) before calling `start_recording`. If denied, emit a `mic-permission-denied` event with a button that opens `x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone`. Also add a 3 s watchdog: if no `audio-chunk` events fire after `start_recording`, surface "No audio detected — check microphone permission".
- **iOS-Relevanz:** iOS-Only — same problem shape, different fix (`AVAudioSession.requestRecordPermission`).
- **Confidence:** High

---

### F4: Hotkey rage-toggle while recording is active leaks audio stream and corrupts UI state
- **Severity (proposed):** High
- **Location:** `src-tauri/src/lib.rs:59-69` (toggle handler) interacts with `src-tauri/src/audio.rs:48` (`IS_RECORDING` atomic) and `app/page.tsx:33-45` (Escape handler)
- **Crash-Mode:** Race / Resource-Leak
- **Trigger:** User presses Cmd+Shift+V to open, hits the mic button, then presses Cmd+Shift+V again to hide. The window hides but `IS_RECORDING` is still true and audio thread keeps draining the buffer + flooding Deepgram.
- **Repro-Snippet:** Cmd+Shift+V (show) → click mic (start) → Cmd+Shift+V (hide window) → wait 30 s → Cmd+Shift+V (re-show) → see stale transcript / battery drain / Deepgram quota burned.
- **Fix-Sketch:** Window-hide handler in `lib.rs` should call `stop_recording` + `stop_deepgram_stream` (or at least pause). Alternatively, treat Cmd+Shift+V as "toggle UI only" but show a banner on hide "Still recording — press Stop to end". Document the chosen behaviour.
- **iOS-Relevanz:** None — no global shortcut on iOS.
- **Confidence:** High

---

### F5: Navigating to Settings during an active recording orphans the transcript event listeners
- **Severity (proposed):** High
- **Location:** `app/hooks/useTauriEvents.ts:161-447` (listeners mount per-component, only on `app/page.tsx`), `app/settings/page.tsx` (no listener)
- **Crash-Mode:** Silent-Loss
- **Trigger:** User starts recording, clicks the gear icon (mid-thought), edits a key, clicks Back.
- **Repro-Snippet:** Click mic → talk for 5 s ("Buy milk") → click Settings cog → say "and eggs" → return to /. The "eggs" portion is dropped because `useTauriEvents` unmounted and the `transcript` event had no listener. Rust still records and bills Deepgram, but the Zustand store never receives the words.
- **Fix-Sketch:** Either (a) put `useTauriEvents` in the root layout so listeners persist across routes, or (b) buffer transcripts in Rust until the frontend re-subscribes (deepgram task already has them — push to a VecDeque and emit on first new listener), or (c) auto-stop recording when navigating away.
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** High

---

### F6: WebSocket disconnect (Wi-Fi off mid-recording) leaves UI in permanent "Listening..." state with no reconnect
- **Severity (proposed):** High
- **Location:** `src-tauri/src/transcription.rs:132-173` (read task exits silently on `Err` or `Close`), `:176-189` (write task exits silently when channel drops)
- **Crash-Mode:** UI-Freeze / Silent-Loss
- **Trigger:** User starts recording on Wi-Fi, walks into a dead zone (or laptop sleeps and wakes), keeps talking.
- **Repro-Snippet:** Start recording on coffee-shop Wi-Fi → router drops → continue speaking. The read task hits `Err(e)` (line 164) and logs to tracing but never emits anything to the UI. `is_streaming` is set false silently. The mic button stays red, "Live" indicator stays on, but no new transcripts ever arrive. `audio.rs:174` keeps dropping audio because `is_streaming = false`.
- **Fix-Sketch:** When the read task exits with an error, emit `deepgram-disconnected { reason }`. Frontend should display a yellow banner "Connection lost — reconnecting…" and attempt one auto-reconnect with the buffered audio. After 3 failures, show "Disconnected — please stop and retry".
- **iOS-Relevanz:** Same-Pfad-iOS — even more frequent on cellular.
- **Confidence:** High

---

### F7: Audio channel back-pressure silently drops samples when Deepgram is slow
- **Severity (proposed):** High
- **Location:** `src-tauri/src/transcription.rs:64-70` (`try_send`), `src-tauri/src/audio.rs:174-178` (`try_lock` + ignored error), channel created with capacity 100 at `transcription.rs:120`
- **Crash-Mode:** Silent-Loss
- **Trigger:** Network jitter, Deepgram momentary slowdown, or any momentary contention on `transcription_state` lock.
- **Repro-Snippet:** Throttle network to "Slow 3G" via Network Link Conditioner → start recording → speak for 30 s. Channel fills (100 × 100 ms = 10 s buffer), `try_send` returns `Err(Full)`, error is discarded with `let _ =`. User sees occasional missing words but no warning. Same hazard with `try_lock` failure at line 174 — entire chunk is dropped if the mutex was held by another task even briefly.
- **Fix-Sketch:** Replace `try_send` with a bounded backlog counter: when 3 consecutive sends fail, emit `transcription-degraded` to the UI. Replace `try_lock` with a blocking `blocking_lock` on the audio thread (it's a dedicated std thread, not async) — the lock is held only for microseconds in the read task.
- **iOS-Relevanz:** Same-Pfad-iOS — worse because mobile networks are more jittery.
- **Confidence:** High

---

### F8: VAD events fire on every audio callback (~50-100 Hz) — IPC flood causes UI jank on low-spec Macs
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/audio.rs:142-146` (emit per callback) and `:181-187` (audio-chunk also every ~100 ms)
- **Crash-Mode:** UI-Freeze
- **Trigger:** Long recording on any machine with limited webview performance (Intel MacBook Air, older M1 with lots of tabs).
- **Repro-Snippet:** Start recording → watch CPU in Activity Monitor → WebContent process spikes. The `vad-event` is emitted for every CPAL callback (default 256-1024 frames at 48 kHz ≈ 5-21 ms intervals) regardless of whether the chunk threshold was reached. Each event serialises through Tauri IPC and triggers a Zustand `setVadState` → React re-render of waveform.
- **Fix-Sketch:** Throttle VAD emission to ≤10 Hz (e.g. only emit when speech state flips OR every Nth callback). Coalesce energy into a rolling average. Same treatment for `audio-chunk` if frontend doesn't actually need raw samples (it doesn't — see `useAudioForwarding` which is a no-op).
- **iOS-Relevanz:** None on the Rust path; iOS web-audio capture has its own throttling.
- **Confidence:** High

---

### F9: Multiple app instances corrupt global shortcut + share RECORDING_BUFFER state
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/lib.rs:32-114` (no single-instance plugin), `src-tauri/src/audio.rs:13` (`static RECORDING_BUFFER`), `:48` (`static IS_RECORDING`)
- **Crash-Mode:** Race
- **Trigger:** User double-clicks the dock icon, or has the app pinned to login and also launches from Spotlight.
- **Repro-Snippet:** Launch app → in Finder, run a second copy → second instance's `register(primary)` fails (already-claimed Cmd+Shift+V) and falls through to fallback Cmd+Shift+A → user now has two windows responding to different shortcuts, both writing to the same Keychain item, but each with its own RECORDING_BUFFER. Save-recording will only see one instance's buffer.
- **Fix-Sketch:** Add `tauri-plugin-single-instance` and on second-launch focus the existing window instead of starting a new app. Document the behaviour in the Info.plist (`LSMultipleInstancesProhibited`).
- **iOS-Relevanz:** None — iOS prohibits multiple instances by design.
- **Confidence:** High

---

### F10: Transcript and RECORDING_BUFFER grow unbounded during long sessions
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/audio.rs:13` (`static RECORDING_BUFFER: Vec<i16>`, never trimmed), `app/store/voiceStore.ts:264-273` (`appendTranscript` concatenates forever)
- **Crash-Mode:** Resource-Leak
- **Trigger:** User leaves recording on during a 1 hour meeting.
- **Repro-Snippet:** Start recording → walk away for 60 min. At 16 kHz × 2 bytes × 3600 s = ~115 MB just for `RECORDING_BUFFER`. The transcript string is also reallocated on every final segment (`transcript + ' ' + text`) — O(n²) memory churn over a long session. No max-duration warning, no rolling buffer.
- **Fix-Sketch:** Add a max recording duration (e.g. 30 min) with a soft warning at 25 min. Cap `RECORDING_BUFFER` to a ring-buffer or auto-flush to disk every N minutes. Use a transcript array internally and `.join(' ')` only at render time.
- **iOS-Relevanz:** Same-Pfad-iOS — much tighter memory budget; iOS background-recording will get OOM-killed quickly.
- **Confidence:** High

---

### F11: Cmd+Q mid-recording loses the in-memory audio buffer and any pending agent calls
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/lib.rs` (no shutdown hook), `src-tauri/src/audio.rs:13` (`RECORDING_BUFFER` is process-memory only), `src-tauri/src/agents/*.rs` (no persistence)
- **Crash-Mode:** Silent-Loss
- **Trigger:** User quits the app (Cmd+Q, force-quit, or system shutdown) while recording or while waiting for the action-items LLM response.
- **Repro-Snippet:** Record a 5 min meeting → click "Action Items" → wait → Cmd+Q before response arrives. Next launch: no transcript, no recording, no action items. The user has lost everything with no warning dialog.
- **Fix-Sketch:** On window-close-request, if `is_recording()` or any agent task is in flight, show a "Discard recording?" dialog. Periodically auto-save the WAV to `app_data_dir()/autosave-<timestamp>.wav` while recording. Persist the most recent transcript to disk (or `localStorage`) so the next launch can offer "Resume last session".
- **iOS-Relevanz:** Same-Pfad-iOS — app suspension by the OS is far more aggressive and unannounced.
- **Confidence:** Medium
