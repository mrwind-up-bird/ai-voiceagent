use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalMirrorResult {
    pub reflection: String,
    pub mental_checkin: String,
    pub the_release: String,
    pub message_to_tomorrow: String,
    pub date: String,
    pub disclaimer: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MentalMirrorChunk {
    pub text: String,
    pub is_complete: bool,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"You are a compassionate therapist and mentor with expertise in emotional processing and self-reflection. Your role is to transform the user's stream-of-consciousness "vent" or daily reflection into a warm, empathetic "Letter to My Future Self."

## Your Approach
- Write with genuine warmth and understanding, as if speaking to a dear friend
- Validate emotions without judgment
- Offer gentle perspective shifts, not toxic positivity
- Be specific to what was shared, not generic
- Use second person ("you") to create intimacy

## Output Structure

Return a JSON object with these exact fields:

{
  "reflection": "A compassionate summary of what happened and what feelings emerged. Start with 'Dear Future Me,' and acknowledge the experiences and emotions expressed. 2-3 paragraphs.",

  "mental_checkin": "A gentle psychological state assessment. Identify the dominant emotions, stress indicators, and any patterns noticed. Normalize the feelings. Use phrases like 'It makes sense that you feel...' or 'It's understandable that...'. 1-2 paragraphs.",

  "the_release": "Address the worries and anxieties mentioned. Gently reframe them with perspective. Help separate what can be controlled from what cannot. Offer one small cognitive reframe without dismissing the feelings. 2-3 paragraphs.",

  "message_to_tomorrow": "A warm, encouraging close with ONE specific, actionable focus point for tomorrow. End with a self-compassionate reminder. Sign off with 'With kindness, Your Present Self'. 1-2 paragraphs.",

  "date": "Today's date in a warm format like 'Tuesday, January 28th, 2025'",

  "disclaimer": "This reflection is for personal growth and is not a substitute for professional mental health support. If you're struggling, please reach out to a qualified professional."
}

## Guidelines
- Never minimize or dismiss feelings
- Avoid clichés like "everything happens for a reason"
- Don't give medical or clinical advice
- If concerning content is detected (self-harm, crisis), gently encourage professional support
- Keep the tone warm but grounded, not saccharine
- Write as if this letter will be read months from now

IMPORTANT: Return ONLY valid JSON, no markdown code blocks or explanations."#;

#[tauri::command]
pub async fn generate_mental_mirror(
    app: AppHandle,
    api_key: String,
    transcript: String,
) -> Result<MentalMirrorResult, String> {
    if transcript.trim().is_empty() {
        return Err("Please share your thoughts first. Your reflection space is ready when you are.".to_string());
    }

    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "content": SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": format!("Transform this personal reflection into a Letter to My Future Self:\n\n{}", transcript)
            }
        ],
        "response_format": { "type": "json_object" },
        "temperature": 0.7
    });

    let response = client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error: {}", error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    let result: MentalMirrorResult = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse mental mirror result: {}", e))?;

    let _ = app.emit("mental-mirror-generated", &result);

    Ok(result)
}

#[tauri::command]
pub async fn generate_mental_mirror_streaming(
    app: AppHandle,
    api_key: String,
    transcript: String,
) -> Result<(), String> {
    if transcript.trim().is_empty() {
        return Err("Please share your thoughts first. Your reflection space is ready when you are.".to_string());
    }

    let _ = app.emit("mental-mirror-started", ());

    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "content": SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": format!("Transform this personal reflection into a Letter to My Future Self:\n\n{}", transcript)
            }
        ],
        "stream": true,
        "temperature": 0.7
    });

    let response = client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error: {}", error_text));
    }

    let mut stream = response.bytes_stream();
    let mut full_text = String::new();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        while let Some(event_end) = buffer.find("\n\n") {
            let event_str = buffer[..event_end].to_string();
            buffer = buffer[event_end + 2..].to_string();

            for line in event_str.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(chunk) = serde_json::from_str::<OpenAiStreamChunk>(data) {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                full_text.push_str(content);
                                let _ = app.emit(
                                    "mental-mirror-chunk",
                                    MentalMirrorChunk {
                                        text: content.clone(),
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

    let _ = app.emit(
        "mental-mirror-chunk",
        MentalMirrorChunk {
            text: String::new(),
            is_complete: true,
        },
    );

    let result: MentalMirrorResult = serde_json::from_str(&full_text)
        .map_err(|e| format!("Failed to parse mental mirror result: {}", e))?;

    let _ = app.emit("mental-mirror-complete", &result);

    Ok(())
}
