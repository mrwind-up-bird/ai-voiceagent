use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneShiftRequest {
    pub text: String,
    pub target_tone: ToneType,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToneType {
    Professional,
    Casual,
    Friendly,
    Formal,
    Empathetic,
    Assertive,
    Diplomatic,
    Enthusiastic,
}

impl ToneType {
    fn description(&self) -> &str {
        match self {
            ToneType::Professional => "professional and business-appropriate",
            ToneType::Casual => "casual and relaxed",
            ToneType::Friendly => "warm and friendly",
            ToneType::Formal => "formal and respectful",
            ToneType::Empathetic => "empathetic and understanding",
            ToneType::Assertive => "confident and assertive",
            ToneType::Diplomatic => "diplomatic and tactful",
            ToneType::Enthusiastic => "enthusiastic and energetic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneShiftResult {
    pub original: String,
    pub shifted: String,
    pub tone: ToneType,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToneShiftChunk {
    pub text: String,
    pub is_complete: bool,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicDelta>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    text: Option<String>,
}

fn build_system_prompt(tone: &ToneType, intensity: u8) -> String {
    let intensity_desc = match intensity {
        1..=3 => "subtle",
        4..=6 => "moderate",
        7..=8 => "strong",
        9..=10 => "maximum",
        _ => "moderate",
    };

    let intensity_guidance = match intensity {
        1..=3 => "Make minimal changes - only adjust a few key words or phrases while keeping most of the original structure intact.",
        4..=6 => "Make balanced changes - adjust vocabulary and some phrasing while maintaining the overall structure.",
        7..=8 => "Make significant changes - substantially rewrite the text with different vocabulary, sentence structure, and expressions.",
        9..=10 => "Make dramatic changes - completely transform the text to strongly embody the target tone, using very different language and style.",
        _ => "Make balanced changes - adjust vocabulary and some phrasing while maintaining the overall structure.",
    };

    format!(
        r#"You are a tone-shifting assistant. Your task is to rewrite text to match a {} tone while preserving the original meaning and key information.

Intensity Level: {} ({}/10)
{}

Guidelines:
- Maintain the core message and all factual content
- Adjust vocabulary, sentence structure, and phrasing to match the target tone
- Keep the text roughly the same length (within 20%)
- Do not add new information or opinions
- Preserve any specific names, dates, or technical terms

Respond with ONLY the rewritten text, no explanations or preamble."#,
        tone.description(),
        intensity_desc,
        intensity,
        intensity_guidance
    )
}

#[tauri::command]
pub async fn shift_tone(
    app: AppHandle,
    api_key: String,
    text: String,
    target_tone: ToneType,
    intensity: Option<u8>,
) -> Result<ToneShiftResult, String> {
    let intensity = intensity.unwrap_or(5).clamp(1, 10);
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 2048,
        "system": build_system_prompt(&target_tone, intensity),
        "messages": [
            {
                "role": "user",
                "content": format!("Rewrite this text in a {} tone (intensity {}/10):\n\n{}", target_tone.description(), intensity, text)
            }
        ]
    });

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error: {}", error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let shifted_text = response_json["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let result = ToneShiftResult {
        original: text,
        shifted: shifted_text,
        tone: target_tone,
        suggestions: vec![],
    };

    let _ = app.emit("tone-shifted", &result);

    Ok(result)
}

#[tauri::command]
pub async fn shift_tone_streaming(
    app: AppHandle,
    api_key: String,
    text: String,
    target_tone: ToneType,
    intensity: Option<u8>,
) -> Result<(), String> {
    let intensity = intensity.unwrap_or(5).clamp(1, 10);
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 2048,
        "stream": true,
        "system": build_system_prompt(&target_tone, intensity),
        "messages": [
            {
                "role": "user",
                "content": format!("Rewrite this text in a {} tone (intensity {}/10):\n\n{}", target_tone.description(), intensity, text)
            }
        ]
    });

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error: {}", error_text));
    }

    let _ = app.emit("tone-shift-started", ());

    let mut stream = response.bytes_stream();
    let mut full_text = String::new();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        // Parse SSE events
        while let Some(event_end) = buffer.find("\n\n") {
            let event_str = buffer[..event_end].to_string();
            buffer = buffer[event_end + 2..].to_string();

            for line in event_str.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        if event.event_type == "content_block_delta" {
                            if let Some(delta) = event.delta {
                                if let Some(text) = delta.text {
                                    full_text.push_str(&text);
                                    let _ = app.emit(
                                        "tone-shift-chunk",
                                        ToneShiftChunk {
                                            text: text.clone(),
                                            is_complete: false,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let _ = app.emit(
        "tone-shift-chunk",
        ToneShiftChunk {
            text: String::new(),
            is_complete: true,
        },
    );

    let result = ToneShiftResult {
        original: text,
        shifted: full_text,
        tone: target_tone,
        suggestions: vec![],
    };

    let _ = app.emit("tone-shift-complete", &result);

    Ok(())
}

#[tauri::command]
pub fn get_available_tones() -> Vec<String> {
    vec![
        "professional".to_string(),
        "casual".to_string(),
        "friendly".to_string(),
        "formal".to_string(),
        "empathetic".to_string(),
        "assertive".to_string(),
        "diplomatic".to_string(),
        "enthusiastic".to_string(),
    ]
}

/// Rich metadata for tone presets
#[derive(Debug, Clone, Serialize)]
pub struct TonePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub use_cases: Vec<String>,
    pub example_before: String,
    pub example_after: String,
    pub icon: String,
    pub color: String,
}

/// Get detailed tone presets with metadata for UI display
#[tauri::command]
pub fn get_tone_presets() -> Vec<TonePreset> {
    vec![
        TonePreset {
            id: "professional".to_string(),
            name: "Professional".to_string(),
            description: "Business-appropriate language suitable for workplace communication".to_string(),
            use_cases: vec![
                "Work emails".to_string(),
                "Client communication".to_string(),
                "Reports".to_string(),
            ],
            example_before: "Hey, can you get that done ASAP?".to_string(),
            example_after: "Could you please prioritize this task at your earliest convenience?".to_string(),
            icon: "briefcase".to_string(),
            color: "#3B82F6".to_string(), // blue
        },
        TonePreset {
            id: "casual".to_string(),
            name: "Casual".to_string(),
            description: "Relaxed and informal language for everyday conversation".to_string(),
            use_cases: vec![
                "Text messages".to_string(),
                "Social media".to_string(),
                "Friends & family".to_string(),
            ],
            example_before: "I would like to inform you that I will be arriving shortly.".to_string(),
            example_after: "Hey, I'll be there soon!".to_string(),
            icon: "chat".to_string(),
            color: "#10B981".to_string(), // green
        },
        TonePreset {
            id: "friendly".to_string(),
            name: "Friendly".to_string(),
            description: "Warm and approachable language that builds rapport".to_string(),
            use_cases: vec![
                "Customer service".to_string(),
                "Welcome messages".to_string(),
                "Team updates".to_string(),
            ],
            example_before: "Your request has been processed.".to_string(),
            example_after: "Great news! We've taken care of your request and you're all set!".to_string(),
            icon: "smile".to_string(),
            color: "#F59E0B".to_string(), // amber
        },
        TonePreset {
            id: "formal".to_string(),
            name: "Formal".to_string(),
            description: "Respectful and structured language for official contexts".to_string(),
            use_cases: vec![
                "Legal documents".to_string(),
                "Academic writing".to_string(),
                "Official letters".to_string(),
            ],
            example_before: "Thanks for your help with this.".to_string(),
            example_after: "I would like to express my sincere gratitude for your assistance in this matter.".to_string(),
            icon: "document".to_string(),
            color: "#6366F1".to_string(), // indigo
        },
        TonePreset {
            id: "empathetic".to_string(),
            name: "Empathetic".to_string(),
            description: "Understanding and compassionate language that acknowledges feelings".to_string(),
            use_cases: vec![
                "Support tickets".to_string(),
                "Difficult conversations".to_string(),
                "Apologies".to_string(),
            ],
            example_before: "We can't refund your purchase.".to_string(),
            example_after: "I understand how frustrating this must be. While we're unable to process a refund, let me see what other options we can explore together.".to_string(),
            icon: "heart".to_string(),
            color: "#EC4899".to_string(), // pink
        },
        TonePreset {
            id: "assertive".to_string(),
            name: "Assertive".to_string(),
            description: "Confident and direct language that commands attention".to_string(),
            use_cases: vec![
                "Negotiations".to_string(),
                "Setting boundaries".to_string(),
                "Leadership comms".to_string(),
            ],
            example_before: "Maybe we could possibly consider changing the deadline?".to_string(),
            example_after: "The deadline needs to be extended. I'll need your confirmation by tomorrow.".to_string(),
            icon: "bolt".to_string(),
            color: "#EF4444".to_string(), // red
        },
        TonePreset {
            id: "diplomatic".to_string(),
            name: "Diplomatic".to_string(),
            description: "Tactful language that navigates sensitive topics gracefully".to_string(),
            use_cases: vec![
                "Feedback".to_string(),
                "Conflict resolution".to_string(),
                "Stakeholder mgmt".to_string(),
            ],
            example_before: "Your idea won't work.".to_string(),
            example_after: "That's an interesting perspective. I'd like to explore some alternative approaches that might address a few concerns.".to_string(),
            icon: "scale".to_string(),
            color: "#8B5CF6".to_string(), // purple
        },
        TonePreset {
            id: "enthusiastic".to_string(),
            name: "Enthusiastic".to_string(),
            description: "Energetic and positive language that inspires excitement".to_string(),
            use_cases: vec![
                "Marketing copy".to_string(),
                "Announcements".to_string(),
                "Motivational".to_string(),
            ],
            example_before: "We have a new product available.".to_string(),
            example_after: "We're thrilled to announce our amazing new product! You're going to love what we've created!".to_string(),
            icon: "sparkles".to_string(),
            color: "#F97316".to_string(), // orange
        },
    ]
}
