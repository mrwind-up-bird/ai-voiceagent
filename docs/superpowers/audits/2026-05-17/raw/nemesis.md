## Nemesis — 2026-05-17
**Scanned:** src-tauri/src/transcription.rs, src-tauri/src/lib.rs, src-tauri/src/secrets.rs, src-tauri/src/audio.rs, src-tauri/src/platform/audio/desktop.rs, src-tauri/src/platform/mod.rs, src-tauri/src/tts.rs, src-tauri/src/sync/{mod,transport,encryption,pairing,webrtc,signaling,discovery,document}.rs, src-tauri/src/agents/{action_items,tone_shifter,music_matcher,translator,mental_mirror,brain_dump,dev_log}.rs, src-tauri/capabilities/{default,mobile}.json, src-tauri/tauri.conf.json
**Findings:** 11

### F1: Audio thread panics on cpal config fallback — kills recording capture
- **Severity (proposed):** High
- **Location:** `src-tauri/src/audio.rs:103` (and twin `src-tauri/src/platform/audio/desktop.rs:84,161,163`)
- **Crash-Mode:** Panic
- **Trigger:** User plugs in / selects a microphone that does not advertise 16 kHz support AND whose default input config is not retrievable (driver hiccup, exclusive-mode access, BT mic that just disconnected). `default_input_config().expect(...)` panics inside the spawned audio capture thread.
- **Repro-Snippet:** Call `start_recording` while the default input device is in a transitional/unavailable state, OR rely on `build_input_stream(...).expect(...)` / `stream.play().expect(...)` failing for any reason (busy device, sample rate not actually buildable). Thread panics, `IS_RECORDING` is never reset because the unwind skips the post-`run_audio_capture` reset path (it only runs on `Err`, not panic).
- **Fix-Sketch:** Convert the three `expect/unwrap` calls in the audio thread to `?` + `Result` propagation, ensure `IS_RECORDING.store(false)` and `recording-error` emit happen in a guard / `Drop` so even a panic leaves the app in a clean state.
- **iOS-Relevanz:** Same-Pfad-iOS (the `platform/audio/desktop.rs` file is desktop-only, but the lib.rs duplicate is compiled the same way)
- **Confidence:** High

### F2: SPAKE2 payload deserialization aborts pairing on any malformed frame
- **Severity (proposed):** High
- **Location:** `src-tauri/src/sync/transport.rs:355-367` (`extract_spake2_payload`) and creator/joiner call sites at lines 187-193, 277-293
- **Crash-Mode:** Silent-Loss
- **Trigger:** A malicious or buggy peer on the LAN (or a man-in-the-middle on the local WebSocket — unauthenticated `ws://` — see F8) sends a non-text frame or a text frame that isn't `{"type":"spake2",...}`. `extract_spake2_payload` errors, the creator/joiner returns, and the higher-level state cleanup runs only via the `handle_creator_connection`'s deferred reset — but `start_creator_transport` does `listener.accept().await` exactly **once** and then returns. There is no retry; one hostile probe burns the whole session.
- **Repro-Snippet:** `nc <creator-ip> <port>` and send a single garbage byte; the creator's accept task ends, the user keeps seeing "WaitingForPeer" until they manually leave.
- **Fix-Sketch:** Loop on `accept` until a successful SPAKE2 exchange (or session timeout); reject and drop the offending socket; log peer addr.
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** High

### F3: Sync `apply_update` accepts arbitrary attacker-chosen yrs updates after SPAKE2 — DoS / state corruption via fuzzed update vector
- **Severity (proposed):** High
- **Location:** `src-tauri/src/sync/transport.rs:519-538` and `src-tauri/src/sync/document.rs:86-92`
- **Crash-Mode:** Silent-Loss / Resource-Leak
- **Trigger:** A peer that has completed SPAKE2 (e.g. the user shared the pairing code in screenshare) sends an encrypted `Update` envelope whose decrypted bytes are *not* parseable as a control `SyncMessage` AND not a valid yrs `Update`. `Update::decode_v1` returns `Err`, `apply_update` logs `warn!` and continues — but the bytes can also be *crafted* yrs updates that bloat the document (yrs has no size cap here). `MAX_SYNC_MESSAGE_SIZE` (5 MB) applies only to the outer ciphertext text frame; a 5 MB ciphertext decrypts to up to 5 MB of attacker-controlled yrs ops appended to the in-memory CRDT, repeated indefinitely. No memory cap on the doc itself.
- **Repro-Snippet:** Paired malicious peer sends repeated 4.9 MB encrypted updates containing many distinct yrs `MapInsert` ops with large string values → unbounded RAM growth in `SyncDocument`.
- **Fix-Sketch:** After successful `apply_update`, enforce a doc-size budget (e.g. encode `state_vector` length or count keys/values) and disconnect on overrun. Also rate-limit inbound update frames.
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** Medium

### F4: Streaming SSE parser uses `String::from_utf8_lossy` then string-splits — multi-byte UTF-8 chunks at boundary cause silent token loss
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/agents/tone_shifter.rs:259-260`, `src-tauri/src/agents/translator.rs:262-263`, `src-tauri/src/agents/mental_mirror.rs:201-202`, `src-tauri/src/agents/brain_dump.rs` (same pattern), `src-tauri/src/agents/dev_log.rs:220`
- **Crash-Mode:** Silent-Loss
- **Trigger:** When `bytes_stream()` returns a chunk that ends mid-UTF-8 codepoint (common with German umlauts ä/ö/ü and emoji), `String::from_utf8_lossy` replaces the partial bytes with `U+FFFD` and the next chunk's leading bytes are also corrupted. For `mental_mirror` and `dev_log` the final result is then `serde_json::from_str(&full_text)` (line 241 / 248) — invalid JSON → hard error path, but the streamed UI chunks were already shown to the user, so the user sees a beautiful reflection and then "Failed to parse mental mirror result" with no recovery.
- **Repro-Snippet:** Trigger a German tone-shift output containing "Größe für süße Möglichkeiten" and observe `from_utf8_lossy` replacing partial multi-byte sequences at network packet boundaries; `mental_mirror_streaming` then fails JSON parse at line 241.
- **Fix-Sketch:** Accumulate a `Vec<u8>` buffer; only decode whole SSE events via `std::str::from_utf8` after locating `\n\n` byte boundaries; keep remaining bytes for the next chunk.
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** High

### F5: `transcribe_with_assemblyai` poll loop blocks Tauri command forever on unexpected status
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/transcription.rs:287-329`
- **Crash-Mode:** UI-Freeze / Resource-Leak
- **Trigger:** AssemblyAI returns a `status` field that is neither `"completed"`, `"error"`, nor any known intermediate value (e.g. a new status like `"throttled"` or the field missing in a malformed JSON response). The `_ => continue,` branch keeps sleeping 1 s — the 120-poll timeout *does* save you, but for two minutes the frontend is stuck with no progress signal and the user cannot cancel. There is no abort handle exposed.
- **Repro-Snippet:** Mock the AssemblyAI endpoint to return `{"status":"queued"}` for >2 min; the user sees no event for 120 s, then a hard timeout error.
- **Fix-Sketch:** Emit a periodic `assemblyai-polling` progress event with `poll_count`; allow a `cancel_transcription` Tauri command that drops the future / cancels the request.
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** Medium

### F6: Deepgram stream task swallows JSON parse errors AND can deadlock if `try_send` channel fills
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/transcription.rs:64-70` (`send_audio_direct` uses `try_send`) and `:154-156` (parse error swallowed)
- **Crash-Mode:** Silent-Loss
- **Trigger:** Deepgram emits unusual frame (server-side feature flag returns a payload Aurus's struct can't deserialize, e.g. a future schema with `metadata` only). The whole frame is dropped with a `warn!` — including final transcripts. Separately, `try_send` on the bounded channel (capacity 100, ~10 s of audio) silently fails under load: when the spawned send task is slow (TLS retransmit), audio drops on the floor and the user's words are silently lost. The frontend never learns.
- **Repro-Snippet:** Simulate a slow WS write (e.g. `tc qdisc add ... delay 2s`); CPAL keeps producing samples at full rate, `try_send` returns `Full`, samples are discarded, no UI signal.
- **Fix-Sketch:** Use blocking `send` from the audio thread (or a larger ring buffer with a drop-oldest policy) AND emit a `audio-dropped` Tauri event when packets are discarded. For parse: gracefully fall back to extracting `channel.alternatives[0].transcript` from a generic `serde_json::Value` if the typed struct fails.
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** High

### F7: `is_creator` field on `SessionEncryption` is unused at decrypt time — direction AAD mismatch is plausible
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/sync/encryption.rs:87-93, 117-140`
- **Crash-Mode:** Silent-Loss
- **Trigger:** `direction_aad(false)` (decrypt) returns the *opposite* of what the peer used to encrypt — this is correct by design, but the test `test_direction_specific_keys` (line 289) shows the code only works because creator's send AAD = `c2j` and joiner's recv AAD = `c2j`. Now consider key rotation: `rotate_key` rotates *both* directions on each side with HKDF info `next-c2j-key` / `next-j2c-key` swapped per side. If both peers issue `KeyRotate` notifications concurrently with different `epoch` values, the receiver runs `rotate_key()` again on a key that was *just* rotated for the outgoing notification — the two sides desync silently because epoch is only checked `> key_rotation_epoch` (transport.rs:503) but the local side increments locally too (transport.rs:647). After concurrent rotations both sides end up at different effective epochs and every subsequent decrypt fails with "Decryption failed — invalid key or tampered data".
- **Repro-Snippet:** Two paired peers, both hit the 30-minute rotation tick within ~1 s of each other. Each sends its own KeyRotate, increments its own counter, processes the peer's KeyRotate. End state: local epoch = N+1 but remote-derived key matches a different schedule. Sync silently dies.
- **Fix-Sketch:** Single side (e.g. creator only) initiates rotations, or both sides agree on a deterministic epoch tied to wall-clock minutes; ignore inbound rotations whose epoch isn't exactly `local_epoch + 1`.
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** Medium

### F8: Local sync WebSocket binds `0.0.0.0` with plaintext `ws://` and trusts unauthenticated callers to SPAKE2
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/sync/transport.rs:121` (bind) + `:266-291` (joiner uses plain `ws://`)
- **Crash-Mode:** Resource-Leak / DoS
- **Trigger:** Hostile host on the same network (coffee shop wifi) port-scans, finds the open WS port, connects, and burns the single `listener.accept()` slot (F2). Even with F2 fixed, an attacker can attempt SPAKE2 with random codes — each attempt costs ~real CPU because Ed25519 group ops are not trivial. No rate limiting; no peer-IP allowlist.
- **Repro-Snippet:** `while true; do nc -w1 <victim-ip> <port>; done` ties up accept + SPAKE2.
- **Fix-Sketch:** Bind to a single interface, prefer `127.0.0.1` when not actively pairing, enforce a 5-attempt-per-minute SPAKE2 rate limit per peer-IP, and drop the listener after first successful pairing.
- **iOS-Relevanz:** Same-Pfad-iOS (mobile also binds via mDNS join path)
- **Confidence:** Medium

### F9: `extract_action_items` returns "Service temporarily unavailable" for *any* JSON parse error — agent silently strips empty content too
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/agents/action_items.rs:100-112`; same pattern in `mental_mirror.rs:127-137`, `dev_log.rs:130-142`, `brain_dump.rs`
- **Crash-Mode:** Silent-Loss
- **Trigger:** GPT-4o returns `{"items":[],"summary":""}` for a transcript with no clear actions — perfectly valid. But it can also return `{"items":[{"task":"...","priority":null,...}]}` where `priority` is null. The struct (`ActionItem`) declares `priority: String` (non-Option) — `serde_json::from_str` fails and the whole response is discarded as a generic parse error. The user gets a friendly "Failed to parse action items: missing field `priority`" leaked to UI (line 112 *does* leak the raw serde error, unlike the API-call branch). Similarly `transcript.is_empty()` is never checked for `action_items` — sending an empty string burns an API call.
- **Repro-Snippet:** Force the model to omit `priority` (rare but happens under temperature drift); user sees a stack-trace-style error.
- **Fix-Sketch:** Make `priority` and `task` use `#[serde(default)]` (or `Option<String>`); validate `transcript.trim().is_empty()` up-front in every agent; never leak raw serde errors to the UI message (they can contain partial transcript content).
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** High

### F10: WebRTC datachannel callback expects mutex never poisoned — `.expect("DcShared mutex poisoned")` panics inside FFI callback thread
- **Severity (proposed):** Medium
- **Location:** `src-tauri/src/sync/webrtc.rs:85, 91, 99, 114, 121`
- **Crash-Mode:** Panic
- **Trigger:** Any panic in another `DcShared` consumer (e.g. an `unbounded_send` after receiver was dropped panicked — won't, but `waker.wake()` could panic if waker is malformed in some custom executor). The `StdMutex` is poisoned; the next datachannel callback (`on_message`, `on_candidate`) calls `.lock().expect(...)` → panic inside libdatachannel's C-thread callback. Unwinding across FFI = undefined behavior → SIGABRT, hard process exit.
- **Repro-Snippet:** Hard to reproduce without injecting a panic; the risk is structural — any panic on any `DcShared` user poisons the mutex permanently, and the FFI callback path has no graceful degradation.
- **Fix-Sketch:** Use `parking_lot::Mutex` (no poison) or `lock().unwrap_or_else(|p| p.into_inner())` to gracefully consume poisoned state inside FFI callbacks.
- **iOS-Relevanz:** None (desktop-only via `#[cfg(not(any(target_os = "ios", target_os = "android")))]`)
- **Confidence:** Medium

### F11: `analyze_mood_from_transcript` & `match_music` accept oversized `MusicMatchRequest.query` and tempo with no length cap
- **Severity (proposed):** Low
- **Location:** `src-tauri/src/agents/music_matcher.rs:75-113` (no length check on `request.query`/`mood`/`genre`/`tempo`); contrast with `:168` which guards `transcript.len() <= 100_000`
- **Crash-Mode:** Resource-Leak
- **Trigger:** Frontend (or compromised renderer) passes a 50 MB string in `MusicMatchRequest.query`. The request is encoded into a URL query string (`client.get(...).query(&query_params)`) — `reqwest` will refuse extremely long URLs but only after allocating; meanwhile Tauri's IPC has already moved the whole string from the renderer to Rust. With smaller-but-still-large strings (~1 MB), the Q-Records API will return 4xx → fallback `create_mock_result(&request.query)` clones the huge `query` into the result and emits it back as a Tauri event — round-trip amplification.
- **Repro-Snippet:** From the frontend devtools: `invoke('match_music', { request: { query: 'a'.repeat(10_000_000) } })`.
- **Fix-Sketch:** Add `MAX_QUERY_LENGTH` (e.g. 4 KB) check at the top of `match_music` and any other agent command that takes user-supplied strings (currently `shift_tone` does, action_items does, but `match_music` and the various `_get_available_*` query inputs do not).
- **iOS-Relevanz:** Same-Pfad-iOS
- **Confidence:** High
