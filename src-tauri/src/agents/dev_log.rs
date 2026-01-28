use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevLogResult {
    pub commit_message: String,
    pub ticket: TicketContent,
    pub slack_update: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketContent {
    pub title: String,
    pub description: String,
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevLogChunk {
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

const SYSTEM_PROMPT: &str = r#"You are a technical documentation expert specializing in transforming messy developer thoughts and voice transcripts into clean, professional documentation.

Your task is to analyze the developer's transcript and generate THREE outputs:

## 1. Conventional Commit Message
Follow the conventional commits specification (https://www.conventionalcommits.org/):
- Format: <type>(<scope>): <description>
- Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore
- Keep the subject line under 72 characters
- Use imperative mood ("add" not "added")
- Include a body if more context is needed

## 2. Jira/Linear Ticket
Create a well-structured ticket with:
- **Title**: Clear, concise summary of the work
- **Description**: Detailed explanation of what needs to be done, why, and any relevant context
- **Acceptance Criteria**: Specific, testable conditions that must be met (as a bulleted list)

## 3. Slack Team Update
Write a brief, friendly update suitable for a team channel:
- Keep it concise (2-4 sentences)
- Mention what was done or what's being worked on
- Include any blockers or next steps if relevant
- Use a professional but approachable tone

Return your response in this exact JSON format:
{
  "commit_message": "type(scope): description\n\nOptional body with more details",
  "ticket": {
    "title": "Clear ticket title",
    "description": "Detailed description of the work",
    "acceptance_criteria": ["Criterion 1", "Criterion 2", "Criterion 3"]
  },
  "slack_update": "Brief team update message"
}

IMPORTANT: Return ONLY valid JSON, no markdown code blocks or explanations."#;

#[tauri::command]
pub async fn generate_dev_log(
    app: AppHandle,
    api_key: String,
    transcript: String,
) -> Result<DevLogResult, String> {
    if transcript.trim().is_empty() {
        return Err("Transcript is empty. Please provide some content.".to_string());
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
                "content": format!("Transform this developer transcript into documentation:\n\n{}", transcript)
            }
        ],
        "response_format": { "type": "json_object" },
        "temperature": 0.3
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

    let result: DevLogResult = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse dev log result: {}", e))?;

    let _ = app.emit("dev-log-generated", &result);

    Ok(result)
}

#[tauri::command]
pub async fn generate_dev_log_streaming(
    app: AppHandle,
    api_key: String,
    transcript: String,
) -> Result<(), String> {
    if transcript.trim().is_empty() {
        return Err("Transcript is empty. Please provide some content.".to_string());
    }

    let _ = app.emit("dev-log-started", ());

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
                "content": format!("Transform this developer transcript into documentation:\n\n{}", transcript)
            }
        ],
        "stream": true,
        "temperature": 0.3
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

        // Parse SSE events
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
                                    "dev-log-chunk",
                                    DevLogChunk {
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
        "dev-log-chunk",
        DevLogChunk {
            text: String::new(),
            is_complete: true,
        },
    );

    // Parse the complete JSON response
    let result: DevLogResult = serde_json::from_str(&full_text)
        .map_err(|e| format!("Failed to parse dev log result: {}", e))?;

    let _ = app.emit("dev-log-complete", &result);

    Ok(())
}
