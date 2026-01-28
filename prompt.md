## Voice Intelligence Hub - Production Implementation Directive

### Role

You are a Lead Full-Stack Engineer specializing in Rust, TypeScript (Next.js 14+), and Agentic AI Systems. Your goal is to implement the core architecture of the "Voice Intelligence Hub" as defined in the Prompt below. You produce production-ready Code. You think pragmatic but out-of-the-box. Always chooses the best approach to deliver production ready Applications.

### Project Overview

You are building a native desktop voice assistant application using Tauri v2 + Next.js 14 (App Router). This is a standalone OS application with a voice-activated Spotlight-like interface triggered by global hotkeys. The architecture strictly separates concerns: Rust handles all system-level operations (audio capture, API calls, secrets management), while Next.js provides a reactive UI compiled to static files.

### Core Architecture Requirements

### Stack

- **Backend:** Rust with Tauri v2, CPAL for audio, ort for local ML inference
- **Frontend:** Next.js 14 App Router with static export (`output: 'export'`)
- **Voice Processing:** Cloud-first (Deepgram Nova-2 primary, AssemblyAI fallback)with local inference fallback (Silero VAD + candle-whisper)
- **LLM Agents:** GPT-4o for structured extraction, Claude 3.5 Sonnet for code/tone tasks
- **Security:** tauri-plugin-stronghold for API keys, keyring-rs for snapshot password
- **Testing:** Vitest for unit tests, Playwright for E2E voice pipeline

### Implementation Roadmap

### Phase 1: Project Scaffolding

```bash
# Initialize Tauri + Next.js
pnpm create tauri-app voice-intelligence-hub
# Select: Next.js, TypeScript, pnpm

# Install dependencies
cd voice-intelligence-hub
pnpm add @tauri-apps/api @tauri-apps/plugin-global-shortcut
pnpm add -D @tauri-apps/cli vitest @testing-library/react playwright
pnpm add zod react-hook-form zustand

# Rust dependencies (add to src-tauri/Cargo.toml)
# cpal = "0.15"
# ort = "2.0"
# reqwest = { version = "0.12", features = ["stream", "json"] }
# tauri-plugin-stronghold = "2.0"
# keyring = "2.3"
# serde = { version = "1.0", features = ["derive"] }
# tokio = { version = "1", features = ["full"] }

```

### Phase 2: Tauri Configuration

**next.config.js**

```jsx
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  images: { unoptimized: true },
  trailingSlash: true,
}
module.exports = nextConfig

```

**src-tauri/capabilities/default.json** - Define ACL permissions

```json
{
  "permissions": [
    "core:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "stronghold:default",
    "shell:allow-execute"
  ]
}

```

### Phase 3: Rust Backend - Core Systems

**Audio Capture (CPAL)**

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::Emitter;

#[tauri::command]
async fn start_audio_capture(app: tauri::AppHandle) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device")?;
    
    let config = device.default_input_config()
        .map_err(|e| e.to_string())?;
    
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _| {
            // Run Silero VAD on chunks
            if is_speech_detected(data) {
                app.emit("voice-detected", data).ok();
            }
        },
        |err| eprintln!("Audio error: {}", err),
        None
    ).map_err(|e| e.to_string())?;
    
    stream.play().map_err(|e| e.to_string())?;
    Ok(())
}

```

**Stronghold Secrets Manager**

```rust
use tauri_plugin_stronghold::StrongholdExt;

#[tauri::command]
async fn store_api_key(
    app: tauri::AppHandle,
    service: String,
    key: String
) -> Result<(), String> {
    let stronghold = app.stronghold();
    stronghold.save_secret(&service, key.as_bytes())
        .await
        .map_err(|e| e.to_string())
}

async fn get_deepgram_key(app: &tauri::AppHandle) -> Result<String, String> {
    let stronghold = app.stronghold();
    let bytes = stronghold.get_secret("deepgram").await?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

```

**Transcription Service (Cloud + Fallback)**

```rust
#[tauri::command]
async fn transcribe_audio(
    app: tauri::AppHandle,
    audio_data: Vec<f32>,
    use_local: bool
) -> Result<String, String> {
    if use_local {
        return local_whisper_transcribe(audio_data).await;
    }
    
    // Try Deepgram first
    match deepgram_transcribe(&app, &audio_data).await {
        Ok(text) => Ok(text),
        Err(_) => {
            // Fallback to AssemblyAI
            assemblyai_transcribe(&app, &audio_data).await
                .or_else(|_| local_whisper_transcribe(audio_data).await)
        }
    }
}

async fn deepgram_transcribe(
    app: &tauri::AppHandle,
    audio: &[f32]
) -> Result<String, String> {
    let key = get_deepgram_key(app).await?;
    let client = reqwest::Client::new();
    // WebSocket streaming implementation
    // ...
}

```

### Phase 4: Agent Implementation

**1. Action Item Ninja (GPT-4o Structured Output)**

```rust
#[derive(Deserialize)]
struct ActionItemResponse {
    tasks: Vec<Task>
}

#[derive(Serialize, Deserialize)]
struct Task {
    description: String,
    priority: String,
    deadline: Option<String>,
    context_quote: String
}

#[tauri::command]
async fn extract_action_items(
    app: tauri::AppHandle,
    transcript: String
) -> Result<Vec<Task>, String> {
    let api_key = get_openai_key(&app).await?;
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "tasks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "description": {"type": "string"},
                        "priority": {"type": "string", "enum": ["High","Medium","Low"]},
                        "deadline": {"type": "string", "format": "date-time"},
                        "context_quote": {"type": "string"}
                    },
                    "required": ["description","priority","context_quote"]
                }
            }
        }
    });
    
    let response: ActionItemResponse = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": transcript}],
            "response_format": {"type": "json_schema", "json_schema": schema}
        }))
        .send().await?
        .json().await?;
    
    Ok(response.tasks)
}

```

**2. Tone Shifter (Claude Streaming)**

```rust
#[tauri::command]
async fn rewrite_professional(
    app: tauri::AppHandle,
    text: String
) -> Result<(), String> {
    let key = get_anthropic_key(&app).await?;
    let mut stream = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "stream": true,
            "messages": [{
                "role": "user",
                "content": format!("Rewrite professionally: {}", text)
            }]
        }))
        .send().await?
        .bytes_stream();
    
    while let Some(chunk) = stream.next().await {
        let text = String::from_utf8_lossy(&chunk?);
        app.emit("tone-shift-chunk", text).ok();
    }
    Ok(())
}

```

**3. Music Matcher (Q-Records API)**

```rust
#[tauri::command]
async fn search_music(
    app: tauri::AppHandle,
    voice_query: String
) -> Result<serde_json::Value, String> {
    
    reqwest::Client::new()
        .post("https://q-records-storemanager.de/query")
        .json(&serde_json::json!({
            "query": voice_query,
            "limit": 10
        }))
        .send().await?
        .json().await
        .map_err(|e| e.to_string())
}

```

### Phase 5: Frontend (Next.js)

**Global Hotkey Handler**

```tsx
// app/hooks/useGlobalShortcut.ts
import { register, unregister } from '@tauri-apps/plugin-global-shortcut';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect } from 'react';

export function useGlobalShortcut(shortcut: string) {
  useEffect(() => {
    const setupHotkey = async () => {
      await register(shortcut, async () => {
        const window = getCurrentWindow();
        const visible = await window.isVisible();
        
        if (visible) {
          await window.hide();
        } else {
          await window.show();
          await window.setFocus();
        }
      });
    };
    
    setupHotkey();
    return () => { unregister(shortcut); };
  }, [shortcut]);
}

```

**Voice Input Component**

```tsx
// app/components/VoiceInput.tsx
'use client';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useState, useEffect } from 'react';

export default function VoiceInput() {
  const [transcript, setTranscript] = useState('');
  const [isRecording, setIsRecording] = useState(false);
  
  useEffect(() => {
    const unlisten = listen('voice-detected', async (event) => {
      const text = await invoke<string>('transcribe_audio', {
        audioData: event.payload,
        useLocal: false
      });
      setTranscript(prev => prev + ' ' + text);
    });
    
    return () => { unlisten.then(fn => fn()); };
  }, []);
  
  const startRecording = async () => {
    setIsRecording(true);
    await invoke('start_audio_capture');
  };
  
  return (
    <div className="voice-input">
      <button onClick={startRecording}>
        {isRecording ? 'Listening...' : 'Speak'}
      </button>
      <p>{transcript}</p>
    </div>
  );
}

```

### Phase 6: Testing

**Unit Tests (Vitest)**

```tsx
// __tests__/agents.test.ts
import { describe, it, expect, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core');

describe('Action Item Extraction', () => {
  it('should extract tasks with correct schema', async () => {
    const mockResponse = [{
      description: 'Review PR',
      priority: 'High',
      context_quote: 'need to review that PR'
    }];
    
    vi.mocked(invoke).mockResolvedValue(mockResponse);
    
    const result = await invoke('extract_action_items', {
      transcript: 'I need to review that PR by Friday'
    });
    
    expect(result).toHaveLength(1);
    expect(result[0].priority).toBe('High');
  });
});

```

**E2E Voice Pipeline (Playwright)**

```tsx
// e2e/voice-flow.spec.ts
import { test, expect } from '@playwright/test';

test('complete voice transcription flow', async ({ page }) => {
  await page.goto('/');
  
  // Simulate global shortcut
  await page.keyboard.press('Control+Shift+Space');
  
  // Wait for window to show
  await expect(page.locator('[data-testid="voice-input"]')).toBeVisible();
  
  // Simulate voice detection event
  await page.evaluate(() => {
    window.__TAURI__.event.emit('voice-detected', {
      payload: new Float32Array(16000)
    });
  });
  
  // Verify transcript appears
  await expect(page.locator('[data-testid="transcript"]'))
    .toContainText('', { timeout: 5000 });
});

```

### Critical Implementation Notes

- **No Server Actions:** All backend logic must use `#[tauri::command]`
- **Static Export:** Verify `pnpm build` generates `out/` directory
- **API Fallbacks:** Every cloud API call must have try-catch with local inference fallback
- **Streaming:** Use Server-Sent Events pattern for LLM responses
- **Memory Safety:** Drop API keys immediately after use in Rust
- **Cross-Platform:** Test hotkeys on all OS (macOS/Windows/Linux)

### Success Criteria

1. Binary size under 10MB (before models)
2. Voice-to-transcript latency under 500ms (cloud mode)
3. 100% test coverage for agent functions
4. Zero API keys exposed to frontend
5. Graceful degradation to local models on network failure

Build production-ready code with proper error handling, TypeScript strict mode, and comprehensive logging. Prioritize security and user privacy at every layer.