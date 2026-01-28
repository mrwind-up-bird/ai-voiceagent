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

fn build_system_prompt(tone: &ToneType) -> String {
    format!(
        r#"You are a tone-shifting assistant. Your task is to rewrite text to match a {} tone while preserving the original meaning and key information.

Guidelines:
- Maintain the core message and all factual content
- Adjust vocabulary, sentence structure, and phrasing to match the target tone
- Keep the text roughly the same length (within 20%)
- Do not add new information or opinions
- Preserve any specific names, dates, or technical terms

Respond with ONLY the rewritten text, no explanations or preamble."#,
        tone.description()
    )
}

#[tauri::command]
pub async fn shift_tone(
    app: AppHandle,
    api_key: String,
    text: String,
    target_tone: ToneType,
) -> Result<ToneShiftResult, String> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 2048,
        "system": build_system_prompt(&target_tone),
        "messages": [
            {
                "role": "user",
                "content": format!("Rewrite this text in a {} tone:\n\n{}", target_tone.description(), text)
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
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 2048,
        "stream": true,
        "system": build_system_prompt(&target_tone),
        "messages": [
            {
                "role": "user",
                "content": format!("Rewrite this text in a {} tone:\n\n{}", target_tone.description(), text)
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
