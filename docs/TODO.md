# TODO - Aurus Voice Intelligence

> **Last Updated:** January 29, 2025

---

## 🔴 Critical (Blockers)

- [ ] **Implement real email service for Mental Mirror**
  - Current: Mock implementation logs to console
  - Options: SendGrid, Resend, AWS SES
  - File: `src-tauri/src/agents/mental_mirror.rs:238`

- [ ] **Fix Q-Records API integration for Music Matcher**
  - Current: Button disabled, backend functions exist
  - Need: Valid API key and endpoint verification
  - Files: `src-tauri/src/agents/music_matcher.rs`, `AgentSelector.tsx`

---

## 🟠 High Priority

### Backend (Rust)

- [ ] **Add error recovery for Deepgram WebSocket disconnects**
  - Auto-reconnect on connection drop
  - Buffer audio during reconnection
  - File: `src-tauri/src/transcription.rs`

- [ ] **Implement API key validation on startup**
  - Check if keys are valid before allowing agent use
  - Show helpful error messages for invalid/expired keys
  - File: `src-tauri/src/secrets.rs`

- [ ] **Add request timeout handling for all AI agents**
  - Current: No timeout, can hang indefinitely
  - Add 30s timeout with user notification
  - Files: All `src-tauri/src/agents/*.rs`

- [ ] **Implement audio device selection**
  - Current: Uses default device
  - Allow user to select input device
  - File: `src-tauri/src/audio.rs`

### Frontend (Next.js)

- [ ] **Add loading states for all agent buttons**
  - Show spinner on active agent
  - Disable other agents during processing
  - File: `app/components/AgentSelector.tsx`

- [ ] **Implement transcript editing**
  - Allow user to correct transcription errors before processing
  - File: `app/components/TranscriptDisplay.tsx`

- [ ] **Add keyboard shortcuts for agents**
  - `Cmd+1` = Action Items, `Cmd+2` = Tone Shifter, etc.
  - File: `app/hooks/useGlobalShortcut.ts`

---

## 🟡 Medium Priority

### Features

- [ ] **Conversation history / session persistence**
  - Store transcripts in SQLite
  - Search past sessions
  - New files needed: `src-tauri/src/database.rs`

- [ ] **Export results to Markdown file**
  - "Save as .md" button for all agents
  - Include metadata (date, agent used, settings)
  - File: `app/components/AgentResults.tsx`

- [ ] **Multi-language transcription**
  - Current: Hardcoded to German (`language=de`)
  - Add language selector in settings
  - File: `src-tauri/src/transcription.rs:45`

- [ ] **Custom tone presets for Tone Shifter**
  - Allow users to save intensity + tone combinations
  - File: `app/components/ToneSelector.tsx`

### Testing

- [ ] **Add integration tests for Tauri commands**
  - Test audio capture mocking
  - Test agent API responses
  - New file: `__tests__/integration/`

- [ ] **Add E2E tests for full recording flow**
  - Record → Transcribe → Process → Display
  - File: `e2e/recording.spec.ts`

- [ ] **Increase test coverage to 80%**
  - Current: ~60% estimated
  - Add tests for all hooks and components

### Performance

- [ ] **Optimize bundle size**
  - Analyze with `next-bundle-analyzer`
  - Lazy load agent result components
  - Target: < 100KB first load JS

- [ ] **Add audio compression before Deepgram**
  - Current: Raw PCM
  - Consider: Opus encoding for lower bandwidth
  - File: `src-tauri/src/audio.rs`

---

## 🟢 Low Priority (Nice to Have)

### UI/UX

- [ ] **Add dark/light theme toggle**
  - Current: Dark mode only
  - File: `tailwind.config.js`, `app/globals.css`

- [ ] **Improve waveform visualization**
  - Current: Simple energy bars
  - Consider: Frequency spectrum display
  - File: `app/components/VoiceInput.tsx`

- [ ] **Add onboarding tutorial**
  - First-run experience explaining features
  - API key setup wizard
  - New component: `app/components/Onboarding.tsx`

- [ ] **Add result animations**
  - Fade-in for streaming text
  - Card entrance animations
  - File: `app/components/AgentResults.tsx`

### Documentation

- [ ] **Create video demo**
  - 2-minute feature overview
  - Upload to YouTube/Loom

- [ ] **Write API documentation**
  - OpenAPI spec for Tauri commands
  - New file: `docs/API.md`

- [ ] **Add inline code comments**
  - Document complex audio processing logic
  - Explain AI prompt engineering decisions

### DevOps

- [ ] **Set up automated releases**
  - Current: Manual `pnpm tauri build`
  - Add: GitHub Actions for multi-platform builds
  - File: `.github/workflows/release.yaml` ✅ (exists, needs testing)

- [ ] **Add crash reporting**
  - Sentry or similar for error tracking
  - User consent for data collection

- [ ] **Create installer packages**
  - macOS: DMG with drag-to-Applications
  - Windows: MSI installer
  - Linux: AppImage + .deb

---

## ✅ Recently Completed

- [x] ~~Implement Mental Mirror (Letter to Myself) agent~~ ✅ Jan 29
- [x] ~~Add email scheduling for Mental Mirror (mock)~~ ✅ Jan 29
- [x] ~~Add file export for Mental Mirror~~ ✅ Jan 29
- [x] ~~Fix translation styling for all languages~~ ✅ Jan 29
- [x] ~~Disable Music Matcher button (backend preserved)~~ ✅ Jan 29
- [x] ~~Dead code cleanup (unused files + dependency)~~ ✅ Jan 29
- [x] ~~Update tests for new agent configuration~~ ✅ Jan 29
- [x] ~~Install letter-for-my-future-self plugin~~ ✅ Jan 29
- [x] ~~Create comprehensive documentation~~ ✅ Jan 29

---

## 📊 Progress Tracking

| Category | Total | Done | Progress |
|----------|-------|------|----------|
| Critical | 2 | 0 | ░░░░░░░░░░ 0% |
| High | 7 | 0 | ░░░░░░░░░░ 0% |
| Medium | 9 | 0 | ░░░░░░░░░░ 0% |
| Low | 10 | 0 | ░░░░░░░░░░ 0% |
| **Total** | **28** | **0** | ░░░░░░░░░░ 0% |

---

## 🏷️ Labels

- `backend` - Rust/Tauri changes
- `frontend` - Next.js/React changes
- `agent` - AI agent improvements
- `audio` - Audio pipeline changes
- `ui` - Visual/UX improvements
- `test` - Testing additions
- `docs` - Documentation
- `devops` - Build/deploy infrastructure

---

## 📝 Notes

### Priorities Explained

- **Critical**: Blocks core functionality or user experience
- **High**: Important for production readiness
- **Medium**: Improves quality but not blocking
- **Low**: Nice to have, do when time permits

### Contributing

When picking up a task:
1. Move to "In Progress" section
2. Create branch: `feat/task-name` or `fix/task-name`
3. Reference this TODO in commit messages
4. Update checkbox when complete

---

*TODO list for Aurus Voice Intelligence - Living document*
