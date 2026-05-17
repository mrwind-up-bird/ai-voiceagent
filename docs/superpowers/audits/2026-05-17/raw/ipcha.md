## Ipcha — 2026-05-17
**Scanned:** src-tauri/src/audio.rs, src-tauri/src/transcription.rs, src-tauri/src/lib.rs, src-tauri/src/agents/{action_items,tone_shifter,music_matcher,brain_dump,dev_log,mental_mirror,translator}.rs
**Findings:** 13

---

### F1: IS_RECORDING latch stuck `true` if cpal callback panics
- **Severity (proposed):** High
- **Location:** `src-tauri/src/audio.rs:72-79` (and indirectly 128-190)
- **Crash-Mode:** Race / UI-Freeze
- **Trigger:** Any panic inside the cpal data callback closure (e.g. F2 device with `channels=0`, or out-of-memory during `Vec::extend` on a giant `data` slice) terminates the thread before reaching `IS_RECORDING.store(false)` on line 78.
- **Repro-Snippet:** Plug in a virtual loopback device that advertises `channels = 0`, call `invoke('start_recording')`. Callback hits `data.chunks(0)` → panic. From then on `is_recording()` returns `true` forever; `start_recording` rejects with "Already recording" until the app is restarted.
- **Fix-Sketch:** Use a `scopeguard`/`Drop` wrapper that resets `IS_RECORDING` on any thread exit (panic or normal). Also reset on `recording-error` emission. Spawn the cpal stream from a struct whose `Drop` impl resets the flag.
- **iOS-Relevanz:** None (audio.rs is desktop-only via cfg).
- **Confidence:** High

---

### F2: `data.chunks(channels)` panics when device reports `channels = 0`
- **Severity (proposed):** High
- **Location:** `src-tauri/src/audio.rs:117, 134-140`
- **Crash-Mode:** Panic
- **Trigger:** `config.channels` is taken straight from cpal with no validation. If a misbehaving/hot-plugged driver yields `channels = 0`, `data.chunks(0)` panics ("chunk size must be non-zero"). On macOS this is unusual but observed with some Aggregate Device configurations and Bluetooth A2DP transitions.
- **Repro-Snippet:** Mock a `cpal::Device` that returns a `StreamConfig` with `channels = 0`. Real-world: rapidly toggle the AirPods between SCO/A2DP profiles while recording.
- **Fix-Sketch:** Validate `channels >= 1` at line 117 and return `Err("Invalid device channel count")`. Also fall back to mono path when `channels == 1` without entering the chunk branch.
- **iOS-Relevanz:** None (cfg-gated desktop).
- **Confidence:** High

---

### F3: `chunk_size = sample_rate / 10` produces zero-sized chunks if `sample_rate = 0`
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/audio.rs:123, 158-159`
- **Crash-Mode:** Resource-Leak / UI-Freeze
- **Trigger:** If cpal hands back a config with `sample_rate = 0` (theoretical but possible with corrupt drivers or AirPlay-style virtual devices), `chunk_size = 0`. Then `buf.len() >= 0` is always true on the next callback invocation, so the inner block drains a zero-length buffer, emits an empty `audio-chunk` event, then loops. Per callback you emit one event; with rapid callbacks the frontend's Tauri event channel saturates.
- **Repro-Snippet:** Force `actual_sample_rate = 0` via a mock host; observe `audio-chunk` events with empty `data` flooding the renderer.
- **Fix-Sketch:** Assert `actual_sample_rate >= 8000` after line 111 and bail with a clear error. Also guard `if chunk_size > 0 && buf.len() >= chunk_size`.
- **iOS-Relevanz:** None.
- **Confidence:** Medium

---

### F4: `recording-started` may fire AFTER `recording-stopped` (or never)
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/audio.rs:69-79, 200, 208, 214-221`
- **Crash-Mode:** Race
- **Trigger:** `start_recording` sets `IS_RECORDING=true` and returns Ok immediately. The cpal stream is built on a separate thread. If `stop_recording` arrives within ~1ms (hotkey spam, debounce miss, automated test), `IS_RECORDING` flips to `false` BEFORE `run_audio_capture` reaches line 200's `emit("recording-started")`. Two failure modes: (a) if stream init succeeds, `recording-started` then `recording-stopped` fire back-to-back with zero audio; (b) if init errors, only `recording-error` fires — frontend never sees `recording-stopped` and remains in "recording" UI state forever.
- **Repro-Snippet:** `for (let i=0;i<100;i++){ invoke('start_recording'); invoke('stop_recording'); }`
- **Fix-Sketch:** Emit `recording-started` synchronously from the `#[tauri::command]` after device validation, OR always emit `recording-stopped` in a `finally`-style block at the end of `run_audio_capture` (replace `let _ = app.emit("recording-stopped", ())` with an unconditional emission via Drop guard).
- **iOS-Relevanz:** None.
- **Confidence:** High

---

### F5: Audio callback holds `buffer_clone` mutex while doing I/O on `RECORDING_BUFFER` and `transcription_state`
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/audio.rs:154-189`
- **Crash-Mode:** Silent-Loss
- **Trigger:** The cpal real-time callback acquires `buffer_clone.lock()` then, INSIDE that lock, acquires `RECORDING_BUFFER.lock()` (line 169) and `transcription_state.try_lock()` (line 174). `save_recording` (line 248) clones the entire `RECORDING_BUFFER` while holding its lock for the duration of a multi-MB clone, then `hound::WavWriter::create` plus per-sample writes ALSO hold the lock indirectly via the held clone. While `save_recording` runs, the audio callback waits → cpal's internal queue overflows → samples are silently dropped by the OS.
- **Repro-Snippet:** Record for 60 seconds, then invoke `save_recording` while still recording. Watch for `audio-chunk` event gaps in the renderer.
- **Fix-Sketch:** Move `RECORDING_BUFFER` writes off the audio thread via an SPSC ring buffer or `crossbeam::queue::ArrayQueue`. Never acquire locks inside the cpal callback — push to a lock-free queue and drain on a separate thread.
- **iOS-Relevanz:** None.
- **Confidence:** High

---

### F6: NaN/Inf f32 samples silently disable VAD
- **Severity (proposed):** Low
- **Location:** `src-tauri/src/audio.rs:50-56, 142-152`
- **Crash-Mode:** Silent-Loss
- **Trigger:** A driver that ever produces a NaN f32 sample poisons `calculate_energy`: `NaN*NaN=NaN`, `sum=NaN`, `(NaN/n).sqrt()=NaN`, `NaN > 0.02 == false`. Result: VAD reports "no speech" for the rest of that callback and any subsequent callback where NaN reappears. `(NaN * 32767.0).clamp(...)` returns NaN, `NaN as i16` is defined to be 0 in Rust, so audio buffer fills with silence.
- **Repro-Snippet:** Inject a single `f32::NAN` into the cpal `data` slice (or `f32::INFINITY`).
- **Fix-Sketch:** After `mono_samples` is built, filter or replace non-finite samples: `if !s.is_finite() { 0.0 } else { s }`. Add a `debug_assert!` and a counter for non-finite samples.
- **iOS-Relevanz:** Same-Pfad-iOS (resampler may exist on iOS too).
- **Confidence:** High

---

### F7: Deepgram `Message::Text` parser is unbounded — JSON-bomb panics the reader task
- **Severity (proposed):** High
- **Location:** `src-tauri/src/transcription.rs:131-173`
- **Crash-Mode:** Panic / Resource-Leak
- **Trigger:** `tungstenite` accepts text frames up to its `max_message_size` default (64 MB). `serde_json::from_str` uses recursive descent; ~10k nested arrays exceed the default stack and abort the reader task. After the panic, the writer task keeps running until `tx` is dropped, but the cleanup that resets `is_streaming = false` (line 172) never executes. `is_streaming` stays `true`; `start_deepgram_stream` rejects future calls.
- **Repro-Snippet:** MITM Deepgram (e.g. via DNS hijack on `api.deepgram.com`) and reply with a WebSocket text frame containing `"["*10000 + "1" + "]"*10000`.
- **Fix-Sketch:** Configure `tungstenite::WebSocketConfig::max_message_size = Some(64 * 1024)` (transcripts are small). Use `serde_json::Deserializer::from_str` with `.disable_recursion_limit(false)` (default 128). Wrap the reader task body in a `tokio::spawn` whose JoinHandle is awaited by a watchdog that resets `is_streaming` on panic.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** High

---

### F8: SSE buffer growth unbounded if delimiter never arrives
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/agents/tone_shifter.rs:257-292` (same pattern in `mental_mirror.rs`, `dev_log.rs`, `brain_dump.rs`, `translator.rs`)
- **Crash-Mode:** Resource-Leak
- **Trigger:** The SSE parser accumulates `chunk_str` into `buffer` and only splits on `"\n\n"`. A malicious/broken proxy can stream gigabytes without ever sending the delimiter; `buffer` grows until OOM. Also `String::from_utf8_lossy(&chunk)` on a chunk that splits a multi-byte UTF-8 boundary injects U+FFFD into the user-facing tone-shift output (silent corruption).
- **Repro-Snippet:** Run a local mitmproxy that intercepts `api.anthropic.com` and streams `"a" * 10_000_000` with no newlines.
- **Fix-Sketch:** Cap `buffer.len()` at e.g. 1 MiB and abort on overflow. Replace the manual UTF-8-lossy concatenation with `bytes` accumulation and only convert to `str` at the delimiter boundary, OR use a proper SSE parser (`eventsource-stream` crate).
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** High

---

### F9: `try_send` on a 100-slot mpsc silently drops audio under backpressure
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/transcription.rs:64-70, 120` and `src-tauri/src/audio.rs:174-178`
- **Crash-Mode:** Silent-Loss
- **Trigger:** The cpal callback uses `send_audio_direct` which uses `try_send` on an mpsc channel of size 100. At 16 kHz / 100ms chunks, that's 10 sec of buffering. If the network stalls (TLS retry, Wi-Fi roam, Deepgram throttle), the writer task falls behind, the queue fills, every subsequent chunk is dropped — and the user has no indication.
- **Repro-Snippet:** Start recording, then disable Wi-Fi for 30 seconds. Re-enable. Compare transcript length to spoken length.
- **Fix-Sketch:** Either (a) bound queue at 2 chunks and surface backpressure as a `transcript-lagging` event, or (b) use an unbounded channel with a `tokio::select!` watchdog that warns when queue depth >50. Never silently drop voice data.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** High

---

### F10: `transcription_state.try_lock()` in audio callback drops first-second audio
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/audio.rs:174-178`, `src-tauri/src/transcription.rs:83-126`
- **Crash-Mode:** Silent-Loss
- **Trigger:** `try_lock` is non-blocking. During `start_deepgram_stream`'s WebSocket handshake (300-1500ms), the lock is held by the connection setup; the audio callback's `try_lock` returns `Err`, the `if state.is_streaming` branch is skipped, and audio is dropped — silently, every callback. The first ~1-2 seconds of speech after pressing record never reach Deepgram.
- **Repro-Snippet:** Press hotkey + immediately speak "one two three". Observe transcript starts at "three".
- **Fix-Sketch:** Decouple the sender Channel from the Mutex-protected state. Keep `mpsc::Sender` in an `ArcSwap<Option<Sender>>` or a separate `Mutex<Option<Sender>>` used only by the audio path; do not hold it during the websocket handshake.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** High

---

### F11: `stop_deepgram_stream` leaks WS reader if server doesn't close
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/transcription.rs:209-222, 132-173`
- **Crash-Mode:** Resource-Leak / Race
- **Trigger:** `stop_deepgram_stream` drops the `Sender` (line 219), which lets the writer task exit and send `Close`. But the reader task only exits when (a) the WebSocket actually closes, or (b) an error arrives. A server that holds the socket open (Deepgram occasionally does on idle) leaves the reader task alive indefinitely, holding the `state_clone` Arc. Restarting via `start_deepgram_stream` then races on the eventually-arriving `state_clone.lock().await` at line 171, which overwrites the fresh `is_streaming=true` with `false`.
- **Repro-Snippet:** `start_deepgram_stream`, `stop_deepgram_stream`, `start_deepgram_stream` — second start may immediately observe `is_streaming=false` mid-recording if the old reader's cleanup fires after the new start.
- **Fix-Sketch:** Explicitly close the WebSocket in `stop_deepgram_stream` (keep a `tokio::sync::oneshot` cancellation token, or a shared `AbortHandle` for both reader and writer tasks). Have the reader's cleanup check a `generation` counter before clobbering shared state.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** Medium

---

### F12: AssemblyAI poll loop hammers endpoint when `status` is absent
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/transcription.rs:288-329`
- **Crash-Mode:** Resource-Leak (cost/rate-limit)
- **Trigger:** If AssemblyAI returns a payload missing `status` (e.g., an error envelope or HTML rate-limit page that happens to parse), `poll_result.status` is `None`, match falls through to `_ => continue`, loop runs 120 times at 1-second intervals — 120 unnecessary requests per failed transcription. Same problem on a 429: code ignores `Retry-After` and just sleeps 1 second.
- **Repro-Snippet:** Mock AssemblyAI to return `{"id":"foo"}` with no status field.
- **Fix-Sketch:** Treat `None` status as an error after N=3 retries. Honor `Retry-After` on 4xx/5xx. Use exponential backoff (1s → 2s → 4s → 8s, capped).
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** High

---

### F13: Empty/whitespace transcripts hit OpenAI for `action_items` and `music_matcher` (no guard)
- **Severity (proposed):** Low
- **Location:** `src-tauri/src/agents/action_items.rs:55-62`, `src-tauri/src/agents/music_matcher.rs:74-78, 164-170`
- **Crash-Mode:** Resource-Leak (cost)
- **Trigger:** Unlike `dev_log`, `brain_dump`, `mental_mirror` (which check `transcript.trim().is_empty()`), `action_items.rs` and `music_matcher.rs::match_music` accept empty/whitespace input and forward it to GPT-4o / Q-Records. GPT-4o then returns a JSON missing `items`/`summary`, which fails `serde_json::from_str::<ActionItemsResult>` — user sees "Failed to parse action items" instead of a clear "no transcript" error. Each call burns ~$0.001 in tokens.
- **Repro-Snippet:** `invoke('extract_action_items', { transcript: '   \n\n   ' })`.
- **Fix-Sketch:** Add `if transcript.trim().is_empty() { return Err("Transcript is empty"); }` to both `extract_action_items` and `extract_action_items_streaming`. Add equivalent guard on `request.query.trim().is_empty()` in `match_music`. Also make `ActionItemsResult::items` default to `Vec::new()` via `#[serde(default)]` to be robust against partial LLM compliance.
- **iOS-Relevanz:** Same-Pfad-iOS.
- **Confidence:** High

---

## Cross-cutting themes
1. **No drop guards on shared state.** `IS_RECORDING`, `TranscriptionState::is_streaming`, and the mpsc channel all rely on happy-path cleanup. Any panic between "set true" and "set false" produces a stuck state. RAII guards everywhere.
2. **Audio thread does I/O.** Three locks + event emission inside the cpal callback is asking for buffer underruns. The real-time invariant ("audio callback must never block") is broken in five places.
3. **Untrusted upstream JSON is parsed without limits.** Deepgram and the SSE agents trust the wire format. A malicious or broken proxy can panic the reader (recursion), exhaust memory (no delimiter), or corrupt user-facing text (UTF-8 boundary splits).
4. **Silent drops everywhere.** `try_send`, `try_lock`, `unwrap_or_default`, `let _ = app.emit(...)` — the system is engineered to never complain, which means failures present as "the AI is bad at transcription today" instead of as actionable errors.
