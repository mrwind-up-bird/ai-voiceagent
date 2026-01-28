use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub task: String,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    pub priority: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItemsResult {
    pub items: Vec<ActionItem>,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"You are an action item extraction agent. Analyze the transcript and extract all action items, tasks, and commitments mentioned.

Return a JSON object with this exact structure:
{
  "items": [
    {
      "task": "Description of the action item",
      "assignee": "Person responsible (or null if not specified)",
      "due_date": "Due date if mentioned (or null)",
      "priority": "high|medium|low based on context",
      "context": "Brief context from the conversation"
    }
  ],
  "summary": "Brief summary of the key takeaways"
}

Be thorough but precise. Only include clear action items, not general discussion points."#;

#[tauri::command]
pub async fn extract_action_items(
    app: AppHandle,
    api_key: String,
    transcript: String,
) -> Result<ActionItemsResult, String> {
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
                "content": format!("Extract action items from this transcript:\n\n{}", transcript)
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

    let openai_response: OpenAiResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let content = openai_response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref())
        .ok_or("No content in response")?;

    let result: ActionItemsResult = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse action items: {}", e))?;

    // Emit event with results
    let _ = app.emit("action-items-extracted", &result);

    Ok(result)
}

#[tauri::command]
pub async fn extract_action_items_streaming(
    app: AppHandle,
    api_key: String,
    transcript: String,
) -> Result<(), String> {
    // For streaming, we'll use the non-streaming endpoint but emit progress
    let _ = app.emit("action-items-processing", ());

    let result = extract_action_items(app.clone(), api_key, transcript).await?;

    let _ = app.emit("action-items-complete", &result);

    Ok(())
}
