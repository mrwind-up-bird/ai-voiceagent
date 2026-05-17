# Crash-Stability Audit & Hardening — Design

**Date:** 2026-05-17
**Repo:** aurus-voiceintelligence (v1.2.0)
**Author:** Brainstorming session (NyxCore + Oli)
**Status:** Draft, pending User-Review

---

## 1. Context & Goal

Aurus is a Tauri v2 + Next.js 14 desktop voice assistant currently shipping as a macOS `.app` / `.dmg`. The user is preparing for daily use across macOS *and* iOS. Prior work has already remediated 32 security findings (commit `c54881d`) and fixed a CSP regression (commit `7b2f117`). Speaker diarization, persona integration, and an iOS build are queued as separate sub-projects.

This spec covers **Sub-Project B: Crash-Stability Audit & Hardening** — the foundation that must be solid before stacking features on top.

**Goal:** Aurus must not hard-crash, silently lose data, or become unrecoverable through normal or adversarial use on macOS today, and the same code paths must be ready to remain crash-safe on iOS tomorrow.

**Out of scope:** new features, refactors not driven by a crash finding, version bump, release packaging, actual iOS build (Sub-Project A).

## 2. Decisions (locked during brainstorming)

| Decision | Choice |
|---|---|
| Audit scope | Full-stack (Rust + Frontend + Tauri IPC boundary) |
| Audit output | Triaged report; Critical+High fixed in-session; Medium+Low → nyxCore Action Points |
| iOS lookahead | Yes — flag iOS-relevant findings now to avoid a second audit pass |
| Fix verification | Full TDD: failing-test-first per fix + property tests for audio parsing |
| Orchestration | Approach A — Parallel persona-lens audit, Cael as judge |
| Commit granularity | One commit per finding |

## 3. Persona Orchestration

Five lens-agents dispatched **in a single tool message** as parallel `Agent` calls. All read-only (no Edit/Write). After return, `Cael` consolidates.

```
Main session
   │ parallel dispatch (one Agent call each, single message)
   ├── Nemesis    (vulnerability-driven panic injection)
   ├── Aletheia   (line-level unwrap/expect/panic/unimplemented)
   ├── Ipcha      (adversarial inputs, red-team)
   ├── Athena     (architecture resilience + iOS lifecycle)
   └── Metis      (pragmatic "what real users hit", shipping readiness)
              │
              ▼
   findings.raw.md (concatenated)
              │
              ▼
   Cael (judge) — dedupe, conflict-resolve, severity-normalize
              │
              ▼
   findings.md (canonical, ranked)
              │
   ┌──────────┴──────────┐
   ▼                     ▼
Critical+High        Medium+Low
TDD-Fix-Loop         nyxCore Action Points
```

### Lens assignments

| Persona | Scope | Probe targets | Out of scope |
|---|---|---|---|
| **Nemesis** | Where can hostile *input* → crash? | `transcription.rs` (Deepgram frame parse, base64/JSON), `agents/*.rs` (LLM response parsing), `secrets.rs` (Keychain errors), Tauri command input signatures in `lib.rs` | Generic OWASP (already covered by prior audit) |
| **Aletheia** | Every `unwrap()`, `expect()`, `panic!()`, `unimplemented!()`, `todo!()`, `.unwrap_or_default()` with data loss | All of `src-tauri/src/`, excluding `#[cfg(test)]` blocks and `tests/` | Frontend (static export) |
| **Ipcha** | Adversarial inputs no linter finds | Malformed audio (0-byte chunks, samplerate=0, NaN floats from resampler), malformed Deepgram WS frames (truncated JSON, wrong `type`), Tauri IPC garbage (giant payloads, JSON bomb, recursive structures), races (e.g. `stop_recording` during `start_recording`) | Code style |
| **Athena** | System-level resilience patterns + iOS lifecycle | React error boundaries per top-level route, Tauri event listener lifecycle in `useTauriEvents.ts`, Deepgram WebSocket reconnect strategy, AVAudioSession interruption path, background task cleanup, memory-pressure handler, mic permission revocation at runtime | Micro-optimizations |
| **Metis** | What *actually* crashes users? Shipping-readiness lens | First-run (all keychain slots empty), settings page with wrong token, weak network (Deepgram mid-stream timeout), hotkey spam, display sleep during recording | Theoretical CVEs without real impact |

### Cael judge rules
1. Cluster findings by `Location` (file + ±5 lines tolerance).
2. Pick the most-detailed finding per cluster as canonical; merge other lenses' `Trigger`/`Repro-Snippet` as extra test vectors.
3. Severity = max across lenses; if spread ≥ 2 levels, annotate `disputed`.
4. `iOS:yes` if any lens marks iOS-relevant.
5. Ranking: by (severity × confidence), but **iOS-only findings pinned to top of their severity band**.

## 4. Findings Schema

### Per-lens output contract

```markdown
## <Persona> — 2026-05-17
**Scanned:** <comma-sep paths>
**Findings:** <N>

### F<n>: <short title>
- **Severity (proposed):** Critical | High | Medium | Low
- **Location:** `src-tauri/src/audio.rs:142`
- **Crash-Mode:** Panic | Silent-Loss | UI-Freeze | Race | Resource-Leak
- **Trigger:** <one-sentence repro>
- **Repro-Snippet:** <smallest input/code path triggering it, test sketch ok>
- **Fix-Sketch:** <1–3 sentences, no code>
- **iOS-Relevanz:** None | Same-Pfad-iOS | iOS-Only
- **Confidence:** High | Medium | Low
```

### File layout

- Raw lens outputs: `docs/superpowers/audits/2026-05-17/raw/<persona>.md`
- Cael-consolidated: `docs/superpowers/audits/2026-05-17/findings.md`
- This spec: `docs/superpowers/specs/2026-05-17-crash-stability-audit-design.md`

### Canonical `findings.md`

```markdown
# Crash-Stability Findings — 2026-05-17

## Critical
| # | Title | Location | Crash-Mode | iOS | Confidence | Lenses | Status |
|---|-------|----------|------------|-----|------------|--------|--------|
| C1 | … | … | … | … | … | … | open |

## High
…

## Medium  → Action Points (nyxCore)
## Low     → Action Points (nyxCore)
```

### Severity rubric

- **Critical:** Hard crash (process exit, panic) in normal user flow
- **High:** Hard crash in edge flow OR silent data loss in normal flow
- **Medium:** UI freeze / inconsistent state, recoverable
- **Low:** Code smell, no user impact

## 5. nyxCore Action Point Pipeline

For each Medium/Low finding, call `mcp__nyxcore__nyxcore_create_action_point` with:
- `projectId: 9dea4fc7-a2e8-4f1b-b30c-2aef7532a772`
- `title`: short title
- `description`: full finding block
- `tags`: `["crash-stability", "<lens>", "audit-2026-05-17"]` + `"iOS"` if applicable
- `severity`: `medium` | `low`

nyxCore is the canonical backlog owner for unfixed findings; `findings.md` records the audit snapshot at this date.

## 6. TDD Fix Loop (Critical + High)

```
1. Failing test first
   - Rust: #[test] / #[tokio::test] under src/<module>/tests or crate-level tests/
   - Frontend: __tests__/<area>.test.ts (vitest)
   - E2E only when crash reproduces solely through UI: e2e/crash-<id>.spec.ts (Playwright)
   - Test MUST be red before fix. Evidence: `cargo test <name>` output showing fail.

2. Minimal fix
   - Smallest diff that turns the test green
   - No unrelated refactor
   - New imports OK; new modules only if necessary

3. Verification (all three green)
   - `cargo test`
   - `cargo clippy -- -D warnings`
   - `pnpm test` (if frontend touched)

4. Nemesis review (dispatched Agent, read-only)
   - Question: was the fix at the right layer? Did it close the input vector or merely catch the panic?
   - If "regression possible" → return to step 1 with extra test vector.

5. Commit per finding
   - Message: `fix(crash): <title> [F<id>]`
   - Body: repro + lens sources + test file path
   - Co-Authored-By trailer per CLAUDE.md convention
```

## 7. Property Tests (Audio Parsing)

Added to `src-tauri/Cargo.toml` under `[dev-dependencies]`: `proptest = "1"`. Default 256 cases per property; seed corpus for known edge inputs (NaN, ±Inf, 0-length, max-length).

| Module | Property |
|---|---|
| `audio.rs` resampler | `forall samples in [0, 10_000_000].map(arbitrary_f32): resample(samples, src_rate, 16_000)` never panics |
| `audio.rs` VAD energy | `forall non-empty frame: energy ≥ 0 && energy.is_finite()` |
| `audio.rs` mono conversion | `forall (n_channels, samples): output.len() == samples.len() / n_channels` |
| `transcription.rs` Deepgram frame parser | `forall json_blob: parse never panics` — returns `Result`, never `unwrap` |

## 8. End-of-Session Smoke

After all Critical+High fixed and green:

```bash
pnpm tauri dev
```

Smoke checklist (recorded in audit-2026-05-17 folder when complete):
- [ ] App starts with empty keychain
- [ ] Settings → enter API keys → save
- [ ] Hotkey Cmd+Shift+V opens window
- [ ] Recording start → transcript appears
- [ ] Mid-stream: Wi-Fi off → app does not freeze, shows error
- [ ] Action items / tone shift / music match all work
- [ ] App can quit + relaunch without state corruption
- [ ] iOS lookahead mental walk-through: AVAudioSession interruption path reviewed

## 9. Definition of Done

1. `findings.md` exists, Cael-judged
2. Each Critical+High has: red test (git history proves it) → green test → own fix commit
3. `cargo test`, `cargo clippy -- -D warnings`, `pnpm test` all green
4. Property tests run ≥ 256 cases without failure
5. Manual smoke completed, checklist boxes checked
6. Medium+Low visible as nyxCore Action Points
7. `CHANGELOG.md` entry under `[Unreleased]` for crash-stability hardening

## 10. Not in Definition of Done

- Version bump (1.2.0 → 1.3.0) — user does this after review
- Release build / DMG — separate sub-project
- iOS build success — Sub-Project A; only lookahead findings here

## 11. Risks & Open Questions

| Risk | Mitigation |
|---|---|
| Audit produces too many Critical findings to fix in one session | Pre-commit gate: if >5 Critical, pause and re-scope with user |
| Cael over-deduplicates and loses a unique repro path | Audit folder keeps raw per-lens outputs untouched for later forensics |
| Full-TDD per fix bloats session length | Critical first, High second; if High count balloons, demote to "test-first, defer property tests" with explicit user nod |
| iOS lookahead findings have no current way to verify (no iOS build yet) | Flag as `iOS-Only` and create nyxCore Action Points tagged for Sub-Project A — never silently fix iOS code paths without ability to test |

## 12. Next Skill After Approval

`superpowers:writing-plans` → step-by-step implementation plan (parallel-dispatch script, per-finding fix-loop, smoke runbook).
