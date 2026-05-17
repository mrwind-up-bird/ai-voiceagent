## Aletheia — 2026-05-17
**Scanned:** `src-tauri/src/**/*.rs` (audio.rs, transcription.rs, lib.rs, tts.rs, secrets.rs, agents/*, platform/{audio,secrets}/*, sync/*)
**Findings:** 11 (after excluding ~80 test-only occurrences in `sync/{encryption,signaling,webrtc,transport,pairing,document,discovery,mod}.rs` and `platform/{audio/desktop,secrets/{apple,linux,windows}}.rs` test modules)

### F1: `expect("DcShared mutex poisoned")` in 5 WebRTC PeerConnection/DataChannel callbacks
- **Severity (proposed):** High
- **Location:** `src-tauri/src/sync/webrtc.rs:85,91,99,114,121`
- **Crash-Mode:** Panic
- **Trigger:** Every sync session that completes a WebRTC negotiation. These run on background datachannel-rs callback threads; if **any** code holding the `StdMutex<DcShared>` ever panics (e.g. an `mpsc::send` on a dropped receiver, an allocation under memory pressure, an `unwrap` deeper in a future poll), the mutex is poisoned and every subsequent ICE/state/message callback aborts the process.
- **Repro-Snippet:** Force a panic in the message-receive path while a sync session is active (e.g. send a deliberately oversized payload that triggers an OOM in `msg.to_vec()`); the next `on_candidate` / `on_message` / `on_connection_state_change` will then crash the whole app.
- **Fix-Sketch:** Replace each `.expect("DcShared mutex poisoned")` with `match … { Ok(g) => g, Err(p) => p.into_inner() }`. The shared state is just channels + a waker; reading after poison is safe. Or wrap each callback body in `std::panic::catch_unwind`.
- **iOS-Relevanz:** None (sync/webrtc is gated `#[cfg(not(target_os = "ios"))]` per CHANGELOG; verify in `sync/mod.rs`).
- **Confidence:** High

### F2: `.expect("Failed to build input stream")` and `.expect("Failed to start audio stream")` in iOS-shared audio capture thread
- **Severity (proposed):** Critical
- **Location:** `src-tauri/src/platform/audio/desktop.rs:161,163`
- **Crash-Mode:** Panic
- **Trigger:** Every recording start. The user clicks record → spawns thread → if CPAL fails to build the input stream (device disconnected between enumeration and build, sample-rate became unsupported, permission revoked mid-session), the entire spawned thread panics. The panic is in `std::thread::spawn` so it does not abort the process, but it silently kills capture with no error path back to the caller — caller already returned `Ok(())` at line 173 before this code even runs.
- **Repro-Snippet:** Unplug USB microphone between `start()` returning Ok and the spawned thread reaching `build_input_stream`; UI shows "recording" forever, no audio captured, no error event emitted.
- **Fix-Sketch:** Build the stream synchronously *before* spawning the thread (so errors propagate via the `Result<(), AudioCaptureError>` return), then move it into the thread only to keep it alive. Or have the thread emit a recording-error event before exiting.
- **iOS-Relevanz:** Same-Pfad-iOS — `platform/audio/desktop.rs` is the desktop trait impl, but the same architectural pattern (panic in spawned thread, silent UI hang) likely exists in any iOS audio impl that follows this scaffold.
- **Confidence:** High

### F3: `.expect("No default input config available for audio device")` in legacy `audio.rs` recording path
- **Severity (proposed):** High
- **Location:** `src-tauri/src/audio.rs:103`
- **Crash-Mode:** Panic
- **Trigger:** Every `start_recording` when 16 kHz is not natively supported AND the device has no default config (rare USB/virtual devices, or a device that vanishes mid-call). Runs inside the spawned recording thread.
- **Repro-Snippet:** Select a virtual audio device (e.g. BlackHole 16ch) configured without a default input config, press record.
- **Fix-Sketch:** Replace `.expect(...)` with `?` and return `Result<cpal::StreamConfig, String>` from a helper; emit `recording-error` event on failure rather than crashing the capture thread.
- **iOS-Relevanz:** None (file is desktop-only per CHANGELOG iOS gating).
- **Confidence:** High

### F4: `.expect("error while running tauri application")` at app top level
- **Severity (proposed):** Low
- **Location:** `src-tauri/src/lib.rs:199`
- **Crash-Mode:** Panic
- **Trigger:** Startup; only fires if Tauri's event loop returns an unrecoverable error. Indistinguishable from a fatal startup error in practice — but emits no telemetry, no log line, no user-facing dialog.
- **Repro-Snippet:** Corrupt `tauri.conf.json` or remove a required permission, then launch.
- **Fix-Sketch:** Replace with explicit `match { … log + std::process::exit(1) }` so logs/telemetry capture the failure before exit.
- **iOS-Relevanz:** Same-Pfad-iOS (`lib.rs` is shared).
- **Confidence:** High

### F5: Silent empty-string fallback for AI-generated content (`as_str().unwrap_or("")`)
- **Severity (proposed):** High
- **Location:** `src-tauri/src/agents/tone_shifter.rs:180-183`, `src-tauri/src/agents/translator.rs:184-187`
- **Crash-Mode:** Silent-Loss
- **Trigger:** Every tone-shift or translate request where the API responds with an unexpected schema (Anthropic or OpenAI changes content shape; safety-block returning `null`; rate-limit error mis-deserialized to JSON; finish_reason "content_filter"). User sees the request "succeed" with an empty `shifted` / `translated` string. Other agents (`brain_dump`, `dev_log`, `music_matcher`) correctly use `.ok_or(...)`; these two are inconsistent.
- **Repro-Snippet:** Mock the API to return `{"content":[{"type":"text","text":null}]}` for tone-shifter or `{"choices":[{"message":{}}]}` for translator; agent emits `tone-shifted` / `translation-complete` with empty payload and returns `Ok`.
- **Fix-Sketch:** Replace both with `.ok_or("API returned no content")?` to match the pattern in `brain_dump.rs:221`, `dev_log.rs:137`, `music_matcher.rs:219`.
- **iOS-Relevanz:** Same-Pfad-iOS (agents are cross-platform).
- **Confidence:** High

### F6: `serde_json::from_str(content)` of LLM output without truncation/timeout guard (paired with F5 pattern in dev_log/brain_dump/music_matcher)
- **Severity (proposed):** Low
- **Location:** `src-tauri/src/agents/brain_dump.rs:225`, `src-tauri/src/agents/dev_log.rs:141`, `src-tauri/src/agents/music_matcher.rs:223`
- **Crash-Mode:** Silent-Loss (returns Err string, but the user-facing message is "Failed to parse")
- **Trigger:** OpenAI returns content that is not valid JSON (e.g. wrapped in markdown fences ```json …```, or partial output due to max_tokens cutoff). These agents request JSON responses, but GPT occasionally violates schema; without a fence-stripper or retry, every such call fails opaquely.
- **Repro-Snippet:** Set `max_tokens` very low and trigger any of these agents — output truncates mid-JSON and parsing fails.
- **Fix-Sketch:** Pre-process `content` to strip ```json fences and trim; on parse error, log the raw content (redacted) for diagnostics.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** Medium

### F7: AssemblyAI silent text fallback (`text.unwrap_or_default()`)
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/transcription.rs:312`
- **Crash-Mode:** Silent-Loss
- **Trigger:** AssemblyAI returns `status: completed` but `text: null` (happens for silence-only uploads, or API regressions). UI emits empty transcript + `confidence: 0.9` as if successful.
- **Repro-Snippet:** Submit silent/sub-VAD-threshold audio via AssemblyAI fallback path.
- **Fix-Sketch:** Treat `text == None` as `Err("AssemblyAI returned no transcript")` rather than silently emitting an empty success event with fake confidence 0.9.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** High

### F8: Sample-count cast `samples.len() as f64` & `(f64 * ratio) as usize` in resampler — truncation on long recordings
- **Severity (proposed):** Low
- **Location:** `src-tauri/src/audio.rs:22,26` and `src-tauri/src/platform/audio/desktop.rs:40,44`
- **Crash-Mode:** Silent-Loss
- **Trigger:** Casts are safe in practice (chunks are 100 ms ≈ 4800 samples), but `as usize` on a negative or NaN `f64` (impossible here, but if `ratio` were ever 0 from a corrupt config) would silently produce a giant index. No bounds check above the `output_len` calculation; if `ratio` is `0.0`, divide-by-zero → infinity → `as usize` → `usize::MAX` → OOM allocation on `Vec::with_capacity`.
- **Repro-Snippet:** Pass `target_rate = 0` to `resample()` — produces `ratio = inf` → `output_len = 0` (actually fine, but worth guarding); the inverse `source_rate = 0` with `target_rate > 0` causes `output_len = (len/inf) as usize = 0`. Real risk: future refactor that swaps args.
- **Fix-Sketch:** Add an `if source_rate == 0 || target_rate == 0 { return samples.to_vec(); }` guard at top of `resample()` (duplicated in both files; consider deduplicating to a single helper).
- **iOS-Relevanz:** Same-Pfad-iOS (`platform/audio/` is the new cross-platform abstraction).
- **Confidence:** Medium

### F9: Sample-rate `as usize` cast on user-influenced device config
- **Severity (proposed):** Low
- **Location:** `src-tauri/src/audio.rs:117,123`, `src-tauri/src/platform/audio/desktop.rs:94,103`
- **Crash-Mode:** Silent-Loss / Resource-Leak
- **Trigger:** `config.channels as usize` and `(actual_sample_rate as usize) / 10`. On 32-bit targets (iOS armv7 legacy, but Tauri v2 only supports arm64 iOS so this is moot), a u32 sample_rate of 192000 fits in usize. Real concern: if `channels == 0` (degenerate config from a broken driver), `data.chunks(0)` panics in `Vec::chunks` (debug) or yields infinite loop (release).
- **Repro-Snippet:** Connect a device whose CPAL reports `channels = 0`. Recording callback hits `data.chunks(channels)` with zero → panic in debug.
- **Fix-Sketch:** Validate `config.channels >= 1` and `actual_sample_rate >= 8000` immediately after selection; return error if not.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** Medium

### F10: `f32 to i16` cast in audio quantization — saturating but loses information silently
- **Severity (proposed):** Low
- **Location:** `src-tauri/src/audio.rs:151`, `src-tauri/src/platform/audio/desktop.rs:131`
- **Crash-Mode:** Silent-Loss (data corruption only audibly: clipping)
- **Trigger:** Every recording. `(s * 32767.0).clamp(-32768.0, 32767.0) as i16` is correct mathematically — clamp ensures `as i16` never overflows. No real bug; documenting it because the brief asks for `f64 as i16` style casts. Confirmed safe due to explicit clamp.
- **Repro-Snippet:** n/a — clamped correctly.
- **Fix-Sketch:** None required. Optionally use `i16::try_from(...).unwrap_or(...)` for clarity but no behavior change.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** High

### F11: `unwrap_or_default()` on `extract_string` for sync document state
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/sync/document.rs:107,172` (and `unwrap_or_else(... "idle" ...)` at 173-174)
- **Crash-Mode:** Silent-Loss
- **Trigger:** Every CRDT snapshot/sync. If a remote peer sends an update whose Yjs map structure deviates from expected (different key type, partial replica, schema migration mismatch), `extract_string` returns `None` → empty transcript silently overwrites local. Coupled with `set_transcript("")` propagation through CRDT, this could erase the user's transcript on the peer.
- **Repro-Snippet:** Have peer A set transcript via `set_transcript("foo")`, then have peer B's `extract_string` return `None` due to type-mismatch (e.g. someone stored a number in `KEY_TRANSCRIPT`). The `snapshot()` then emits `transcript: ""` to the frontend, overwriting `voiceStore`.
- **Fix-Sketch:** Return `Option<String>` from `get_transcript()` and have the snapshot consumer (Tauri event handler) distinguish "no transcript yet" from "empty transcript". Avoid auto-defaulting required session fields.
- **iOS-Relevanz:** Same-Pfad-iOS (sync is cross-platform per recent CHANGELOG, though datachannel is desktop-gated; document.rs itself compiles on iOS).
- **Confidence:** Medium

---

**Excluded as test-only (verified `#[cfg(test)]` boundaries):**
- `sync/encryption.rs:235-359` (all unwraps after line 228)
- `sync/signaling.rs:294-325` (after 263)
- `sync/webrtc.rs:570-609` and `panic!`s 575/592/609 (all after 561)
- `sync/transport.rs:690-751` incl. `panic!`s (after 681)
- `sync/pairing.rs:114-166` (after 99)
- `sync/document.rs:280-329` (after 256)
- `sync/discovery.rs:164` (after 155)
- `sync/mod.rs:789` (after 780)
- `platform/audio/desktop.rs:208-221` (after 204)
- `platform/secrets/{apple,linux,windows}.rs` all `.expect(...)` calls (test modules)
