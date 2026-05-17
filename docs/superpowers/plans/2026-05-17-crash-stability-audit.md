# Crash-Stability Audit & Hardening — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden Aurus against crashes across Rust backend, Frontend, and Tauri IPC boundary using parallel persona-lens audit and per-finding TDD fixes, with iOS lookahead.

**Architecture:** Phase 1 dispatches 5 read-only lens-agents in parallel. Phase 2 has Cael consolidate findings. Phase 3 sets up `proptest` and writes property tests for audio parsing (these are always written, not gated on findings). Phase 4 is a per-finding TDD loop (red test → minimal fix → green → Nemesis review → commit). Phase 5 does end-of-session smoke + nyxCore Action Points for Medium/Low + CHANGELOG.

**Tech Stack:** Rust (Tauri v2, tokio, cpal, proptest), TypeScript (Next.js 14 static export, vitest, Playwright), Tauri IPC, nyxCore MCP.

---

## File Structure

Plan-owned files (created or modified during execution):

| Path | Responsibility |
|---|---|
| `docs/superpowers/audits/2026-05-17/raw/nemesis.md` | Nemesis raw findings |
| `docs/superpowers/audits/2026-05-17/raw/aletheia.md` | Aletheia raw findings |
| `docs/superpowers/audits/2026-05-17/raw/ipcha.md` | Ipcha raw findings |
| `docs/superpowers/audits/2026-05-17/raw/athena.md` | Athena raw findings |
| `docs/superpowers/audits/2026-05-17/raw/metis.md` | Metis raw findings |
| `docs/superpowers/audits/2026-05-17/findings.md` | Cael-consolidated canonical findings |
| `docs/superpowers/audits/2026-05-17/smoke.md` | Smoke checklist after fixes |
| `src-tauri/Cargo.toml` | Add `proptest` to `[dev-dependencies]` |
| `src-tauri/tests/property_audio.rs` | Property tests for `audio.rs` (resampler, VAD, mono) |
| `src-tauri/tests/property_transcription.rs` | Property tests for Deepgram frame parser |
| `src-tauri/src/**` | Per-finding fixes (paths TBD by audit) |
| `src-tauri/tests/crash_*.rs` | Per-finding regression tests |
| `__tests__/crash_*.test.ts` | Frontend per-finding regression tests (if applicable) |
| `CHANGELOG.md` | Append `[Unreleased]` hardening entry |

---

## Phase 1 — Parallel Persona-Lens Audit (Read-Only)

### Task 1: Dispatch 5 lens-agents in parallel

**Files:**
- Create: `docs/superpowers/audits/2026-05-17/raw/{nemesis,aletheia,ipcha,athena,metis}.md`

**Discipline:** Single tool message containing 5 `Agent` calls, run concurrently. Each agent must be read-only (Read/Grep/Glob/Bash for `cargo check`/`grep` only — NO Edit/Write). Output format must match the per-lens output contract in the spec.

- [ ] **Step 1: Dispatch all 5 agents in one message**

Agent calls (one tool message, 5 calls):

1. `Agent(description="Nemesis vuln scan", subagent_type="general-purpose", prompt=<nemesis prompt>)` — see prompt below
2. `Agent(description="Aletheia line scan", ...)` — line-level unwrap/expect/panic hunt
3. `Agent(description="Ipcha red-team", ...)` — adversarial inputs
4. `Agent(description="Athena arch+iOS", ...)` — architecture resilience + iOS lifecycle
5. `Agent(description="Metis pragmatic", ...)` — what users actually crash

**Nemesis prompt template:**
```
You are Nemesis, the vulnerability-driven crash auditor for Aurus Voice
Intelligence (Tauri v2 + Next.js). Read-only — do NOT Edit or Write.

Scan these files for crash paths triggered by hostile input:
- src-tauri/src/transcription.rs (Deepgram frame parsing, base64/JSON)
- src-tauri/src/agents/*.rs (LLM response parsing)
- src-tauri/src/secrets.rs (Keychain API errors)
- Tauri command signatures in src-tauri/src/lib.rs (#[tauri::command])

Output a single markdown report to:
  docs/superpowers/audits/2026-05-17/raw/nemesis.md

Use the EXACT schema below. Report ONLY crash-inducing findings, not
generic OWASP (a 32-finding security audit already ran). Aim for max 10
high-quality findings, not noise.

## Nemesis — 2026-05-17
**Scanned:** <files>
**Findings:** <N>

### F1: <title>
- **Severity (proposed):** Critical | High | Medium | Low
- **Location:** `path:line`
- **Crash-Mode:** Panic | Silent-Loss | UI-Freeze | Race | Resource-Leak
- **Trigger:** <one sentence>
- **Repro-Snippet:** <smallest input/code path>
- **Fix-Sketch:** <1-3 sentences, no code>
- **iOS-Relevanz:** None | Same-Pfad-iOS | iOS-Only
- **Confidence:** High | Medium | Low

Be terse. Concrete file:line citations. No essays.
```

Similar prompts for the other four (see spec Section 3 lens assignments table).

- [ ] **Step 2: Verify all 5 raw files exist**

```bash
ls -1 docs/superpowers/audits/2026-05-17/raw/
```
Expected output: `aletheia.md athena.md ipcha.md metis.md nemesis.md`

- [ ] **Step 3: Commit raw audit outputs**

```bash
git add docs/superpowers/audits/2026-05-17/raw/
git commit -m "audit(crash): raw lens outputs from 5 personas

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 2 — Cael Consolidation

### Task 2: Cael judge → canonical findings.md

**Files:**
- Create: `docs/superpowers/audits/2026-05-17/findings.md`

- [ ] **Step 1: Dispatch Cael judge agent**

```
Agent(description="Cael judge", subagent_type="general-purpose", prompt=<cael prompt>)
```

Cael prompt:
```
You are Cael, the dual-provider judge consolidating crash-audit
findings. Read-only — do NOT Edit or Write source code; you only
Write the consolidated findings.md.

Inputs to read:
  docs/superpowers/audits/2026-05-17/raw/nemesis.md
  docs/superpowers/audits/2026-05-17/raw/aletheia.md
  docs/superpowers/audits/2026-05-17/raw/ipcha.md
  docs/superpowers/audits/2026-05-17/raw/athena.md
  docs/superpowers/audits/2026-05-17/raw/metis.md

Algorithm:
1. Cluster findings by Location (file + ±5 lines tolerance).
2. Per cluster: pick the most detailed finding as canonical; merge
   other lenses' Trigger/Repro-Snippet as extra test vectors.
3. Severity = max across lenses; if spread ≥ 2 levels, mark
   `disputed: <lenses>`.
4. iOS-Tag if any lens flagged iOS-relevant.
5. Ranking: by (severity_numeric × confidence_numeric), but
   iOS-Only findings pinned to top of their severity band.

Severity rubric:
- Critical: hard crash (process exit, panic) in normal user flow
- High: hard crash in edge flow OR silent data loss in normal flow
- Medium: UI freeze / inconsistent state, recoverable
- Low: code smell, no user impact

Write output to: docs/superpowers/audits/2026-05-17/findings.md

Schema:
# Crash-Stability Findings — 2026-05-17

## Critical
| # | Title | Location | Crash-Mode | iOS | Confidence | Lenses | Status |
|---|-------|----------|------------|-----|------------|--------|--------|
| C1 | … | … | … | … | … | … | open |

## High
…

## Medium
…

## Low
…

After the tables, include a "## Details" section with full repro
and fix-sketch per Critical+High finding (the M/L ones go to nyxCore).
```

- [ ] **Step 2: Read findings.md, count Critical+High**

```bash
grep -E "^\| (C|H)" docs/superpowers/audits/2026-05-17/findings.md | wc -l
```

Decision gate: if Critical > 5, pause and report back to user. Otherwise proceed.

- [ ] **Step 3: Commit consolidated findings**

```bash
git add docs/superpowers/audits/2026-05-17/findings.md
git commit -m "audit(crash): cael-consolidated findings

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 3 — Property Test Foundation (always)

### Task 3: Add proptest dependency

**Files:**
- Modify: `src-tauri/Cargo.toml` (under `[dev-dependencies]`)

- [ ] **Step 1: Read current Cargo.toml**

```bash
grep -n "dev-dependencies" src-tauri/Cargo.toml
```

- [ ] **Step 2: Add proptest**

Add line under `[dev-dependencies]`:
```toml
proptest = "1"
```

- [ ] **Step 3: Verify build**

```bash
cd src-tauri && cargo check --tests
```
Expected: `Finished` without errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore(test): add proptest for property-based audio tests

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 4: Property tests for audio.rs

**Files:**
- Create: `src-tauri/tests/property_audio.rs`

- [ ] **Step 1: Read audio.rs to find public function signatures**

```bash
grep -nE "^pub (fn|async fn)" src-tauri/src/audio.rs
```

- [ ] **Step 2: Write property tests**

Create `src-tauri/tests/property_audio.rs` with proptest strategies for:
- Resampler: any f32 input → never panics
- VAD energy: non-empty frame → energy ≥ 0 && finite
- Mono conversion: interleaved samples → output length invariant

(Concrete test code is written based on actual public API discovered in Step 1; do not invent functions that don't exist.)

- [ ] **Step 3: Run property tests**

```bash
cd src-tauri && cargo test --test property_audio
```
Expected: 256+ cases pass per property.

If a property fails: that's a finding. Add to `findings.md` under Critical (proptest found a real panic), then run the per-finding TDD loop (Task 6 pattern).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/property_audio.rs
git commit -m "test(audio): property tests for resampler, VAD, mono conversion

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 5: Property tests for Deepgram frame parser

**Files:**
- Create: `src-tauri/tests/property_transcription.rs`

Same pattern as Task 4 but for the Deepgram WebSocket message parser. Property: `parse_deepgram_frame(any json bytes)` returns `Result<…, _>` and never panics. (Concrete signatures discovered from `transcription.rs`.)

- [ ] **Step 1-4: Same TDD pattern**

---

## Phase 4 — Per-Finding TDD Fix Loop (Critical + High)

### Task 6+: Template — apply once per Critical/High finding

This task is **instantiated N times**, once per Critical+High finding from `findings.md`. Each instance gets its own commit.

For each finding `Fi`:

**Files:**
- Create: `src-tauri/tests/crash_F<i>_<slug>.rs` (or `__tests__/crash_F<i>_<slug>.test.ts` if frontend)
- Modify: the file:line cited in the finding

- [ ] **Step 1: Read findings.md Details for F<i>**

Locate the Details block for `F<i>`; read Trigger, Repro-Snippet, Fix-Sketch.

- [ ] **Step 2: Write failing test**

For Rust: create `src-tauri/tests/crash_F<i>_<slug>.rs` containing the smallest test that reproduces the crash. The test should panic / fail / time out BEFORE the fix.

For frontend: create `__tests__/crash_F<i>_<slug>.test.ts`.

Code must be concrete — no placeholders. Derive inputs from the Repro-Snippet field.

- [ ] **Step 3: Run test to verify it's red**

```bash
cd src-tauri && cargo test --test crash_F<i>_<slug>
```
Expected: FAIL or PANIC. If it accidentally passes, the test isn't reproducing the bug — rewrite before fixing.

- [ ] **Step 4: Apply minimal fix at file:line from finding**

The fix should:
- Be the smallest diff that turns the test green
- Match the Fix-Sketch in the finding
- Not include unrelated refactors

- [ ] **Step 5: Run targeted test + full suite**

```bash
cd src-tauri && cargo test --test crash_F<i>_<slug>
cd src-tauri && cargo test
cd src-tauri && cargo clippy -- -D warnings
cd /Users/oli/Projects/aurus-voiceintelligence && pnpm test
```
All four must be green.

- [ ] **Step 6: Nemesis review of the fix**

Dispatch a fresh Nemesis read-only agent:
```
Agent(description="Nemesis review F<i>",
      prompt="Read the diff at <commit-pending> for finding F<i> at
              <file:line>. Question: is the fix at the right layer?
              Did it close the input vector or merely catch the panic?
              Are there nearby code paths with the same flaw? Return:
              'approved' OR 'regression-risk: <reason> | extra-test: <vector>'.")
```

If Nemesis returns `regression-risk`:
- Add an additional test vector matching the suggestion to the test file
- Re-run from Step 4

If Nemesis returns `approved`: proceed.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/tests/crash_F<i>_<slug>.rs <modified-source-files>
git commit -m "fix(crash): <finding title> [F<i>]

Repro: <one line from Trigger>
Lenses: <comma-sep>
Test: src-tauri/tests/crash_F<i>_<slug>.rs

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 8: Update findings.md status**

In `docs/superpowers/audits/2026-05-17/findings.md`, change the `Status` column for `F<i>` from `open` to `fixed:<short-commit-sha>`.

Repeat Task 6 for every Critical+High finding.

---

## Phase 5 — Backlog, Smoke, Closeout

### Task 7: Push Medium+Low to nyxCore as Action Points

**Files:** none modified (MCP-only)

- [ ] **Step 1: Read all Medium+Low entries from findings.md**

- [ ] **Step 2: For each, call**

```
mcp__nyxcore__nyxcore_create_action_point(
  projectId="9dea4fc7-a2e8-4f1b-b30c-2aef7532a772",
  title=<finding title>,
  description=<full finding block>,
  tags=["crash-stability","<lens>","audit-2026-05-17"] + (["iOS"] if applicable),
  severity=<medium|low>
)
```

- [ ] **Step 3: Verify count matches**

```
mcp__nyxcore__nyxcore_search(
  projectId="9dea4fc7-a2e8-4f1b-b30c-2aef7532a772",
  query="audit-2026-05-17"
)
```
Expected: result count == count of M+L findings in `findings.md`.

### Task 8: Manual smoke checklist

**Files:**
- Create: `docs/superpowers/audits/2026-05-17/smoke.md`

- [ ] **Step 1: Write smoke checklist file**

Contents = the checklist from spec Section 8, plus space to record outcome.

- [ ] **Step 2: Cannot execute smoke autonomously (requires UI interaction)**

Mark each box as `[deferred-to-user]` and explain in the file that this requires hands-on launch (`pnpm tauri dev`). The session ends with the smoke deferred; the user runs the checklist when back.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/audits/2026-05-17/smoke.md
git commit -m "docs(audit): smoke checklist (deferred for user)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 9: CHANGELOG entry

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Read top of CHANGELOG.md**

```bash
head -20 CHANGELOG.md
```

- [ ] **Step 2: Append [Unreleased] section if missing**

Add (or extend) `## [Unreleased]` with subsection:
```markdown
### Security & Stability
- Crash-stability audit (2026-05-17): N findings, X Critical/High fixed in-session, Y deferred to backlog.
- Property tests for audio resampler, VAD, mono conversion.
- Property tests for Deepgram frame parser.
```

(Numbers come from actual audit.)

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record crash-stability audit hardening

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Definition of Done (mirrors spec §9)

- [ ] `findings.md` exists, Cael-judged
- [ ] Each Critical+High has red→green test history + own fix commit
- [ ] `cargo test`, `cargo clippy -- -D warnings`, `pnpm test` all green
- [ ] Property tests pass ≥ 256 cases
- [ ] Medium+Low pushed to nyxCore as Action Points
- [ ] `smoke.md` exists with user-deferred checklist
- [ ] `CHANGELOG.md` updated under `[Unreleased]`

## Out of Scope (mirrors spec §10)

- Version bump 1.2.0 → 1.3.0
- Release build / DMG
- iOS build success (Sub-Project A)
- Refactors not driven by a finding
