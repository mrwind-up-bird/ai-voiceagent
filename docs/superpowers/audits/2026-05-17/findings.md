# Crash-Stability Findings — 2026-05-17

**Audit lenses:** Nemesis, Aletheia, Ipcha, Athena, Metis
**Judge:** Cael
**Total raw findings:** 57  → **Consolidated:** 43
**Counts:** 7 Critical · 11 High · 18 Medium · 7 Low

> **Triage decision (2026-05-17):** C1 and C2 are iOS-Only architectural items requiring Tauri Mobile lifecycle infrastructure not yet present in this codebase. They are **deferred to Sub-Project A (iOS build)** as nyxCore Action Points; they are NOT fixed in this session because no iOS test target exists to verify the fix. C3–C7 and all H1–H11 are fixed in-session with TDD.

## Critical
| # | Title | Location | Crash-Mode | iOS | Conf | Lenses | Status |
|---|-------|----------|------------|-----|------|--------|--------|
| C1 | No AVAudioSession interruption handling (calls/Siri kill capture forever) | architectural; `src-tauri/src/platform/audio/mobile.rs` is `NotSupported` stub | Silent-Loss | iOS-Only | High | Athena | deferred-to-subA |
| C2 | No iOS background-suspension lifecycle hook | architectural; no `applicationDidEnterBackground` registration | Silent-Loss | iOS-Only | High | Athena | deferred-to-subA |
| C3 | Audio capture thread panics on cpal `expect/unwrap` (default_input_config, build_input_stream, stream.play) | `src-tauri/src/audio.rs:103,128-190`, `src-tauri/src/platform/audio/desktop.rs:84,161,163` | Panic | Same-Path | High | Nemesis, Aletheia, Ipcha, Athena | open |
| C4 | First-run with zero API keys looks identical to a working app — user records into a void | `app/page.tsx:22-219`, `app/components/VoiceInput.tsx:88-110`, `src-tauri/src/secrets.rs:89` (`has_api_keys` never called from frontend) | Silent-Loss | Same-Path | High | Metis | open |
| C5 | Wrong/expired API keys yield only generic "Service temporarily unavailable" — no 401/429/5xx distinction | `src-tauri/src/agents/*.rs`, `transcription.rs:109-117`, `app/components/VoiceInput.tsx:104-106` | Silent-Loss | Same-Path | High | Metis | open |
| C6 | Microphone permission denied/revoked at OS level → red mic button, zero callbacks, no error | `src-tauri/src/audio.rs:59-110` (no permission probe); architectural cross-platform | Silent-Loss | Same-Path | High | Metis, Athena | open |
| C7 | Deepgram WebSocket has no reconnect / backoff — any transient net event drops transcripts silently forever | `src-tauri/src/transcription.rs:131-173` + `:175-189` | Silent-Loss | Same-Path | High | Athena, Metis | open |

## High
| # | Title | Location | Crash-Mode | iOS | Conf | Lenses | Status |
|---|-------|----------|------------|-----|------|--------|--------|
| H1 | WebRTC `DcShared` mutex `.expect("poisoned")` in 5 FFI callbacks → SIGABRT on unwind across FFI | `src-tauri/src/sync/webrtc.rs:85,91,99,114,121` | Panic | no | High | Aletheia, Nemesis | open |
| H2 | SPAKE2 listener `accept()` single-shot — one hostile probe burns the pairing session | `src-tauri/src/sync/transport.rs:121,187-193,266-291,355-367` | Silent-Loss | Same-Path | High | Nemesis | open |
| H3 | Audio capture has no supervisor; CPAL stream-error callback only logs (mic unplug → UI hangs in "recording") | `src-tauri/src/audio.rs:72-82,191-194,203-205` | Silent-Loss | Same-Path | High | Athena | open |
| H4 | `try_send` on 100-slot mpsc + `try_lock` silently drop audio under backpressure | `src-tauri/src/transcription.rs:64-70,120`, `src-tauri/src/audio.rs:174-178` | Silent-Loss | Same-Path | High | Nemesis, Ipcha, Metis | open |
| H5 | Hotkey rage-toggle while recording: Cmd+Shift+V hide leaks recording stream + Deepgram quota | `src-tauri/src/lib.rs:59-69` ↔ `src-tauri/src/audio.rs:48`; `app/page.tsx:33-45` | Race / Resource-Leak | no | High | Metis | open |
| H6 | Navigating to Settings during recording unmounts `useTauriEvents` — transcript events orphaned | `app/hooks/useTauriEvents.ts:161-447`, `app/settings/page.tsx` | Silent-Loss | Same-Path | High | Metis | open |
| H7 | `RECORDING_BUFFER` + `transcript` grow unbounded — OOM/jetsam on long sessions | `src-tauri/src/audio.rs:13`, `app/store/voiceStore.ts:264-273` | Resource-Leak | Same-Path | High | Athena, Metis (disputed: Metis=Medium) | open |
| H8 | Silent empty-string fallback in agents (`as_str().unwrap_or("")`) — succeeds with empty payload on API schema drift | `src-tauri/src/agents/tone_shifter.rs:180-183`, `translator.rs:184-187`, `action_items.rs:100-112` | Silent-Loss | Same-Path | High | Aletheia, Nemesis, Ipcha (disputed: Ipcha=Low, Aletheia=High) | open |
| H9 | No React Error Boundary anywhere in the tree — any render exception unmounts entire app | `app/layout.tsx`, `app/page.tsx` (architectural) | UI-Freeze | Same-Path | High | Athena | open |
| H10 | Deepgram `Message::Text` parser unbounded — JSON-bomb panics reader, leaves `is_streaming=true` stuck | `src-tauri/src/transcription.rs:131-173` | Panic / Resource-Leak | Same-Path | High | Ipcha | open |
| H11 | Sync `apply_update` accepts attacker-crafted yrs updates after SPAKE2 — unbounded CRDT memory growth | `src-tauri/src/sync/transport.rs:519-538`, `src-tauri/src/sync/document.rs:86-92` | Silent-Loss / Resource-Leak | Same-Path | Medium | Nemesis | open |

## Medium
| # | Title | Location | Crash-Mode | iOS | Conf | Lenses | Status |
|---|-------|----------|------------|-----|------|--------|--------|
| M1 | SSE parser: `String::from_utf8_lossy` + unbounded buffer (UTF-8 boundary corruption + OOM if no delimiter) | `src-tauri/src/agents/tone_shifter.rs:259-260`, `translator.rs:262-263`, `mental_mirror.rs:201-202`, `brain_dump.rs`, `dev_log.rs:220` | Silent-Loss / Resource-Leak | Same-Path | High | Nemesis, Ipcha | open |
| M2 | AssemblyAI poll loop: unknown status burns 120 requests; no Retry-After; no cancel; emits empty text as `confidence=0.9` | `src-tauri/src/transcription.rs:287-329,312` | UI-Freeze / Silent-Loss | Same-Path | High | Nemesis, Ipcha, Aletheia | open |
| M3 | Deepgram parse errors swallowed; final transcripts silently dropped on schema drift | `src-tauri/src/transcription.rs:154-156` | Silent-Loss | Same-Path | High | Nemesis | open |
| M4 | SPAKE2 key-rotation epoch desync on concurrent rotations — sync silently dies | `src-tauri/src/sync/encryption.rs:87-93,117-140`, `src-tauri/src/sync/transport.rs:503,647` | Silent-Loss | Same-Path | Medium | Nemesis | open |
| M5 | Local sync WS binds `0.0.0.0` plaintext with no rate limit on SPAKE2 attempts | `src-tauri/src/sync/transport.rs:121,266-291` | Resource-Leak / DoS | Same-Path | Medium | Nemesis | open |
| M6 | All `reqwest::Client::new()` calls lack `connect_timeout`/`timeout` — hung streams freeze UI forever | `src-tauri/src/transcription.rs:238`, `src-tauri/src/agents/*.rs` | UI-Freeze | Same-Path | High | Athena | open |
| M7 | voiceStore `recordingState` desyncs from Rust `IS_RECORDING`/`is_streaming` — Stop returns "Not recording" | `app/store/voiceStore.ts:99,254` ↔ `audio.rs` / `transcription.rs` | Race | Same-Path | High | Athena | open |
| M8 | `useTauriEvents` async race: unmount before `listen()` resolves leaks subscriptions | `app/hooks/useTauriEvents.ts:161-447` | Resource-Leak | Same-Path | Medium | Athena | open |
| M9 | `transcribe_local_whisper` cfg-gated off on iOS — no documented offline fallback | `src-tauri/src/transcription.rs:337-469`, `src-tauri/src/lib.rs:150-151` | Silent-Loss | iOS-Only | Medium | Athena | open |
| M10 | VAD events emitted per-callback (~50-100 Hz) flood Tauri IPC → WebContent jank | `src-tauri/src/audio.rs:142-146,181-187` | UI-Freeze | no | High | Metis | open |
| M11 | Multiple app instances share Keychain/`RECORDING_BUFFER` and steal each other's shortcuts | `src-tauri/src/lib.rs:32-114`, `src-tauri/src/audio.rs:13,48` | Race | no | High | Metis | open |
| M12 | Cmd+Q mid-recording loses entire in-memory buffer + pending agent calls; no autosave | `src-tauri/src/lib.rs` (no shutdown hook), `src-tauri/src/audio.rs:13` | Silent-Loss | Same-Path | Medium | Metis | open |
| M13 | Audio callback acquires 3 locks + does I/O (`save_recording` clones MB-sized buffer under lock) | `src-tauri/src/audio.rs:154-189,248` | Silent-Loss | no | High | Ipcha | open |
| M14 | `recording-started` event can fire after `recording-stopped` (or never) on rapid toggles | `src-tauri/src/audio.rs:69-79,200,208,214-221` | Race | no | High | Ipcha | open |
| M15 | `chunk_size = sample_rate / 10` = 0 if driver returns `sample_rate=0` → IPC flood of empty chunks | `src-tauri/src/audio.rs:123,158-159` | Resource-Leak / UI-Freeze | no | Medium | Ipcha | open |
| M16 | `stop_deepgram_stream` leaks reader task when server holds socket open; next start races on cleanup | `src-tauri/src/transcription.rs:209-222,132-173` | Resource-Leak / Race | Same-Path | Medium | Ipcha | open |
| M17 | `transcription_state.try_lock()` drops first ~1-2 s of audio during WS handshake | `src-tauri/src/audio.rs:174-178`, `src-tauri/src/transcription.rs:83-126` | Silent-Loss | Same-Path | High | Ipcha | open |
| M18 | `sync::document::extract_string().unwrap_or_default()` overwrites peer transcript with empty string on schema mismatch | `src-tauri/src/sync/document.rs:107,172-174` | Silent-Loss | Same-Path | Medium | Aletheia | open |

## Low
| # | Title | Location | Crash-Mode | iOS | Conf | Lenses | Status |
|---|-------|----------|------------|-----|------|--------|--------|
| L1 | NaN/Inf f32 samples poison VAD (`(NaN*NaN).sqrt() > threshold` is false; `NaN as i16 = 0`) | `src-tauri/src/audio.rs:50-56,142-152` | Silent-Loss | Same-Path | High | Ipcha | open |
| L2 | `resample()` divide-by-zero if `source_rate==0` or `target_rate==0` → `output_len = usize::MAX` allocation | `src-tauri/src/audio.rs:22,26`, `src-tauri/src/platform/audio/desktop.rs:40,44` | Silent-Loss / Resource-Leak | Same-Path | Medium | Aletheia | open |
| L3 | `data.chunks(channels)` panics if driver reports `channels=0` (e.g. AirPods SCO/A2DP toggle) | `src-tauri/src/audio.rs:117,134-140`, `platform/audio/desktop.rs:94,103` | Panic | no | High | Ipcha, Aletheia | open |
| L4 | `lib.rs:199` `.expect("error while running tauri application")` — no log/telemetry on fatal startup | `src-tauri/src/lib.rs:199` | Panic | Same-Path | High | Aletheia | open |
| L5 | `match_music` / `analyze_mood_from_transcript` accept oversized `query`/`mood`/`genre`/`tempo` with no length cap | `src-tauri/src/agents/music_matcher.rs:75-113` | Resource-Leak | Same-Path | High | Nemesis | open |
| L6 | LLM JSON output not fence-stripped — markdown ```json wrappers fail `serde_json::from_str` | `src-tauri/src/agents/brain_dump.rs:225`, `dev_log.rs:141`, `music_matcher.rs:223` | Silent-Loss | Same-Path | Medium | Aletheia | open |
| L7 | `action_items` / `match_music` accept empty/whitespace transcript and burn API tokens | `src-tauri/src/agents/action_items.rs:55-62`, `music_matcher.rs:74-78,164-170` | Resource-Leak | Same-Path | High | Ipcha | open |

---

## Details

### C1: No AVAudioSession interruption handling (iOS-Only) — DEFERRED to Sub-Project A
- **Location:** Architectural — `src-tauri/src/platform/audio/mobile.rs` is a `NotSupported` stub
- **Defer reason:** No iOS build target yet; fix requires Tauri Mobile lifecycle infrastructure that does not exist in this codebase. Will be addressed by Sub-Project A.
- **Fix-Sketch (for Sub-Project A):** `PausableAudioPipeline` trait with `pause()`/`resume()`. iOS impl registers for `AVAudioSession.interruptionNotification`. Pause/restart Deepgram WS on resume (combine with C7 fix).

### C2: No iOS background-suspension lifecycle hook (iOS-Only) — DEFERRED to Sub-Project A
- **Location:** Architectural — no `applicationDidEnterBackground` hook
- **Defer reason:** Same as C1.
- **Fix-Sketch (for Sub-Project A):** On `applicationWillResignActive` gracefully stop, persist, mark `paused`. On `applicationDidBecomeActive` re-validate mic perm, WS, secrets.

### C3: Audio capture thread panics on cpal `expect/unwrap`
- **Location:** `src-tauri/src/audio.rs:103,128-190`; twin `src-tauri/src/platform/audio/desktop.rs:84,161,163`
- **Crash-Mode:** Panic (spawned thread; `IS_RECORDING` never reset)
- **iOS:** Same-Path (scaffold pattern propagates to any future iOS impl)
- **Confidence:** High
- **Lenses:** Aletheia=Critical, Nemesis=High, Ipcha=High, Athena=High
- **Disputed?:** yes — spread of 1 between lenses
- **Trigger:** User selects a mic that doesn't advertise 16 kHz support AND whose default input config is not retrievable. Three `.expect()` calls panic: `default_input_config()`, `build_input_stream(...)`, `stream.play()`. Panic skips `IS_RECORDING.store(false)`.
- **Repro:** Unplug USB mic between `start()` returning `Ok` and thread reaching `build_input_stream` → UI shows "recording" forever.
- **Extra test vectors:** AirPods SCO/A2DP toggle while recording; driver returns `channels=0` (see L3); stream-error callback during normal op (H3); IS_RECORDING stuck-true after panic.
- **Fix-Sketch:** Build cpal stream synchronously BEFORE spawning capture thread so `?` propagates. RAII guard resets `IS_RECORDING` on panic/Err. Always emit `recording-error`.

### C4: First-run with zero API keys looks identical to a working app
- **Location:** `app/page.tsx:22-219`, `app/components/VoiceInput.tsx:88-110`, `src-tauri/src/secrets.rs:89`
- **Crash-Mode:** Silent-Loss
- **iOS:** Same-Path
- **Confidence:** High
- **Lenses:** Metis=Critical
- **Trigger:** Fresh install → click big mic → talk → nothing happens → mic stays red.
- **Extra test vectors:** Deepgram key set but no agent keys → record works, agents fail with C5. Delete a key in Settings → no re-check on return.
- **Fix-Sketch:** On `app/page.tsx` mount, invoke `has_api_keys`. If false, route to `/settings` with banner. Disable record until at least one transcription provider configured. Re-check on focus/route-return.

### C5: Wrong/expired API keys yield generic "Service temporarily unavailable"
- **Location:** `src-tauri/src/agents/*.rs`, `transcription.rs:109-117`, `app/components/VoiceInput.tsx:104-106`
- **Crash-Mode:** Silent-Loss
- **iOS:** Same-Path
- **Confidence:** High
- **Lenses:** Metis=Critical
- **Trigger:** Typo'd or expired API key, then any feature use.
- **Extra test vectors:** HTTP 402 (billing); HTTP 429 indistinguishable from auth; Deepgram WS close 1008/4xxx breaks read loop.
- **Fix-Sketch:** Distinguish 401/403 (auth) from 429 (rate) from 5xx (outage). Parse Deepgram close frame reason; emit `deepgram-auth-failed`. In `VoiceInput.tsx`, if `start_deepgram_stream` rejects, do NOT call `start_recording`; show red toast linking to Settings.

### C6: Microphone permission denied/revoked at OS level — silent failure
- **Location:** `src-tauri/src/audio.rs:59-110` (no permission probe); architectural cross-platform
- **Crash-Mode:** Silent-Loss / UI-Freeze
- **iOS:** Same-Path
- **Confidence:** High
- **Lenses:** Metis=Critical, Athena=High
- **Disputed?:** yes (spread = 1)
- **Trigger:** macOS user with mic disabled clicks record → button red, zero callbacks. Mid-session revoke → zero-filled buffers (macOS/Windows) or fail-to-activate (iOS).
- **Extra test vectors:** First-launch tap before OS prompt granted; resume from iOS background with permission revoked (combine C2); Windows model same.
- **Fix-Sketch:** Pre-flight via `AVAudioApplication.requestRecordPermission`. On denial, emit `mic-permission-denied` with deep-link to Privacy settings. 3 s watchdog: if no `audio-chunk` after `start_recording`, surface "No audio detected".

### C7: Deepgram WebSocket has no reconnect / backoff
- **Location:** `src-tauri/src/transcription.rs:131-173` + `:175-189`
- **Crash-Mode:** Silent-Loss / UI-Freeze
- **iOS:** Same-Path
- **Confidence:** High
- **Lenses:** Athena=Critical, Metis=High
- **Disputed?:** yes (spread = 1)
- **Trigger:** Wi-Fi handoff/VPN reconnect/Deepgram timeout — receive loop `break`s. `is_streaming=false`, but audio capture keeps running; `try_send` on disconnected channel silently succeeds. UI shows mic moving + "Live" indicator but no transcripts.
- **Extra test vectors:** Coffee-shop Wi-Fi drops; laptop sleep/wake; Deepgram idle timeout; cellular handoff (iOS).
- **Fix-Sketch:** Supervised actor with exponential backoff (250ms → 8s, max 5). Emit `deepgram-disconnected`/`-reconnecting`/`-reconnect-failed`. Tear down audio capture if reconnect exhausts. After 3 failures show "Disconnected — please stop and retry".

### H1: WebRTC `DcShared` mutex `.expect("poisoned")` in 5 FFI callbacks
- **Location:** `src-tauri/src/sync/webrtc.rs:85,91,99,114,121`
- **Crash-Mode:** Panic (unwinding across FFI = UB → SIGABRT)
- **iOS:** no (sync/webrtc cfg-gated off for iOS/Android)
- **Confidence:** High
- **Lenses:** Aletheia=High, Nemesis=Medium (spread = 1)
- **Trigger:** Sync session completes WebRTC negotiation; callbacks on background FFI threads. Any panic in code holding the mutex poisons it permanently.
- **Fix-Sketch:** Replace `.expect(...)` with `match ... { Ok(g) => g, Err(p) => p.into_inner() }`. Shared state is channels + waker; reading after poison is safe.

### H2: SPAKE2 listener `accept()` single-shot
- **Location:** `src-tauri/src/sync/transport.rs:121,187-193,266-291,355-367`
- **Crash-Mode:** Silent-Loss
- **iOS:** Same-Path
- **Confidence:** High
- **Lenses:** Nemesis=High
- **Trigger:** `start_creator_transport` does `listener.accept().await` exactly once. Hostile/buggy peer sends garbage → accept task ends. UI stuck in "WaitingForPeer".
- **Fix-Sketch:** Loop on `accept` until successful SPAKE2 (or session timeout); reject/drop offending socket; log peer addr. Combine with M5 rate-limiting per peer-IP.

### H3: Audio capture has no supervisor — stream-error callback only logs
- **Location:** `src-tauri/src/audio.rs:72-82,191-194,203-205`
- **Crash-Mode:** Silent-Loss
- **iOS:** Same-Path (AVAudioSession interruptions surface here — see C1)
- **Confidence:** High
- **Lenses:** Athena=High
- **Trigger:** Error callback only logs — doesn't flip `IS_RECORDING`, doesn't emit `recording-error`. Device unplug / mid-session revoke / underrun leaves UI stuck.
- **Fix-Sketch:** Supervised task with watchdog. Error callback signals `tokio::sync::Notify` triggering restart or clean teardown with `recording-error`. Sleep loop should check stream-healthy flag, not just `IS_RECORDING`.

### H4: `try_send` + `try_lock` silently drop audio under backpressure
- **Location:** `src-tauri/src/transcription.rs:64-70,120`, `src-tauri/src/audio.rs:174-178`
- **Crash-Mode:** Silent-Loss
- **iOS:** Same-Path (worse on jittery mobile networks)
- **Confidence:** High
- **Lenses:** Metis=High, Nemesis=Medium, Ipcha=Medium (spread = 1)
- **Trigger:** Network stall → 100-slot queue fills → every subsequent chunk dropped. `try_lock` failure during WS handshake (M17) discards chunk. Both swallowed by `let _ =`.
- **Fix-Sketch:** Replace `try_send` with backlog counter — emit `transcription-degraded`/`audio-dropped` after ≥3 consecutive `Full`. Replace `try_lock` with `blocking_lock` on dedicated audio thread.

### H5: Hotkey rage-toggle while recording leaks audio + quota
- **Location:** `src-tauri/src/lib.rs:59-69` ↔ `src-tauri/src/audio.rs:48`; `app/page.tsx:33-45`
- **Crash-Mode:** Race / Resource-Leak
- **iOS:** no
- **Confidence:** High
- **Lenses:** Metis=High
- **Trigger:** Cmd+Shift+V open → click mic → Cmd+Shift+V hide. Window hides but `IS_RECORDING` stays true.
- **Fix-Sketch:** Window-hide handler calls `stop_recording` + `stop_deepgram_stream`.

### H6: Settings navigation orphans transcript listeners
- **Location:** `app/hooks/useTauriEvents.ts:161-447`, `app/settings/page.tsx`
- **Crash-Mode:** Silent-Loss
- **iOS:** Same-Path
- **Confidence:** High
- **Lenses:** Metis=High
- **Trigger:** User starts recording, clicks gear, returns. Transcripts emitted between unmount/remount are lost.
- **Fix-Sketch:** Either (a) put `useTauriEvents` in root layout, (b) buffer transcripts in Rust until frontend re-subscribes, or (c) auto-stop recording on navigate-away.

### H7: Unbounded `RECORDING_BUFFER` + `transcript`
- **Location:** `src-tauri/src/audio.rs:13`, `app/store/voiceStore.ts:264-273`
- **Crash-Mode:** Resource-Leak (OOM / iOS jetsam)
- **iOS:** Same-Path (much worse on iOS)
- **Confidence:** High
- **Lenses:** Athena=High, Metis=Medium (spread = 1)
- **Trigger:** Buffer grows ~32 KB/s; 1h ≈ 115 MB. Transcript reallocated on every final → O(n²).
- **Fix-Sketch:** Cap `RECORDING_BUFFER` to rolling N-min window OR stream-append to disk every ~1 min. Cap `transcript` length OR use array + `.join(' ')` at render. iOS memory-warning flushes both.

### H8: Silent empty-string fallback in agents
- **Location:** `src-tauri/src/agents/tone_shifter.rs:180-183`, `translator.rs:184-187`, `action_items.rs:100-112`
- **Crash-Mode:** Silent-Loss
- **iOS:** Same-Path
- **Confidence:** High
- **Lenses:** Aletheia=High, Nemesis=Medium, Ipcha=Low (spread = 2)
- **Trigger:** API schema drift / safety-block / `finish_reason: content_filter` returns `null` content. User sees "success" with empty payload.
- **Fix-Sketch:** Replace unwraps with `.ok_or("API returned no content")?` matching the pattern in `brain_dump:221`/`dev_log:137`/`music_matcher:219`. Validate `transcript.trim().is_empty()` upfront (also fixes L7).

### H9: No React Error Boundary anywhere
- **Location:** `app/layout.tsx`, `app/page.tsx` (architectural)
- **Crash-Mode:** UI-Freeze (white screen, no recovery)
- **iOS:** Same-Path (critical on iOS WebView)
- **Confidence:** High
- **Lenses:** Athena=High
- **Trigger:** Any uncaught render-time exception unmounts entire app.
- **Fix-Sketch:** Top-level `<ErrorBoundary>` in `app/layout.tsx` with "Something went wrong — Reset" fallback calling `voiceStore.reset()`. Per-feature boundary around agent-result components.

### H10: Deepgram parser unbounded — JSON-bomb panics reader
- **Location:** `src-tauri/src/transcription.rs:131-173`
- **Crash-Mode:** Panic / Resource-Leak
- **iOS:** Same-Path
- **Confidence:** High
- **Lenses:** Ipcha=High
- **Trigger:** `tungstenite` default max-message-size 64 MB; `serde_json` recursive descent ~10k nested arrays exceeds default stack. Cleanup at line 172 never runs; `is_streaming` stuck true.
- **Fix-Sketch:** `WebSocketConfig::max_message_size = Some(64 * 1024)`. `serde_json::Deserializer` with default recursion limit. Wrap reader in spawned task with watchdog resetting `is_streaming` on panic.

### H11: Sync `apply_update` accepts crafted yrs updates after SPAKE2
- **Location:** `src-tauri/src/sync/transport.rs:519-538`, `src-tauri/src/sync/document.rs:86-92`
- **Crash-Mode:** Silent-Loss / Resource-Leak
- **iOS:** Same-Path
- **Confidence:** Medium
- **Lenses:** Nemesis=High
- **Trigger:** Paired peer sends 4.9 MB encrypted update with many distinct yrs ops → unbounded RAM growth.
- **Fix-Sketch:** Enforce doc-size budget after `apply_update`; disconnect on overrun. Rate-limit inbound update frames. Drop peer after N parse failures.

---

## Routing summary

| Severity | Count | Destination |
|---|---|---|
| Critical (C1, C2) | 2 | nyxCore Action Points, tagged `iOS-Only` and `subA-deferred` |
| Critical (C3–C7) | 5 | TDD fix loop, this session |
| High (H1–H11) | 11 | TDD fix loop, this session |
| Medium (M1–M18) | 18 | nyxCore Action Points |
| Low (L1–L7) | 7 | nyxCore Action Points |
