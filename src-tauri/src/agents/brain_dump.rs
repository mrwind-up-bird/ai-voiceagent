use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainDumpResult {
    pub tasks: Vec<Task>,
    pub creative_ideas: Vec<CreativeIdea>,
    pub notes: Vec<Note>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub title: String,
    pub description: String,
    pub quadrant: EisenhowerQuadrant,
    pub due_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EisenhowerQuadrant {
    UrgentImportant,      // Do First
    NotUrgentImportant,   // Schedule
    UrgentNotImportant,   // Delegate
    NotUrgentNotImportant, // Eliminate/Later
}

impl EisenhowerQuadrant {
    pub fn label(&self) -> &str {
        match self {
            EisenhowerQuadrant::UrgentImportant => "Do First",
            EisenhowerQuadrant::NotUrgentImportant => "Schedule",
            EisenhowerQuadrant::UrgentNotImportant => "Delegate",
            EisenhowerQuadrant::NotUrgentNotImportant => "Later",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            EisenhowerQuadrant::UrgentImportant => "#EF4444",      // red
            EisenhowerQuadrant::NotUrgentImportant => "#3B82F6",   // blue
            EisenhowerQuadrant::UrgentNotImportant => "#F59E0B",   // amber
            EisenhowerQuadrant::NotUrgentNotImportant => "#6B7280", // gray
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeIdea {
    pub title: String,
    pub description: String,
    pub category: Option<String>,
    pub potential: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub content: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrainDumpChunk {
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

const SYSTEM_PROMPT: &str = r#"You are a cognitive organization expert specializing in processing unstructured thoughts and voice memos. Your task is to analyze a brain dump transcript and categorize its contents.

## Categories

### 1. Tasks (Eisenhower Matrix)
Identify actionable items and classify them using the Eisenhower Matrix:
- **urgent_important**: Crisis, deadlines, problems requiring immediate attention → "Do First"
- **not_urgent_important**: Planning, improvement, learning, relationships → "Schedule"
- **urgent_not_important**: Interruptions, some meetings, some calls → "Delegate"
- **not_urgent_not_important**: Time wasters, pleasant activities, trivia → "Later"

For each task, extract:
- A clear, actionable title
- Brief description with context
- The appropriate quadrant
- Any time hints mentioned (e.g., "by Friday", "next week")

### 2. Creative Ideas
Capture any creative thoughts, inspirations, or brainstorming:
- Business ideas
- Project concepts
- Solutions to problems
- Artistic or creative expressions
- "What if" scenarios

For each idea, provide:
- A catchy title
- Description of the idea
- Category (optional: business, personal, tech, art, etc.)
- Potential impact or value (optional)

### 3. General Notes
Everything else that doesn't fit tasks or ideas:
- Observations
- Reminders without clear actions
- Information to remember
- Thoughts and reflections

For each note, provide:
- The content
- Relevant tags for organization

## Output Format

Return your response in this exact JSON format:
{
  "tasks": [
    {
      "title": "Clear action item",
      "description": "Context and details",
      "quadrant": "urgent_important|not_urgent_important|urgent_not_important|not_urgent_not_important",
      "due_hint": "by Friday" or null
    }
  ],
  "creative_ideas": [
    {
      "title": "Idea name",
      "description": "Full description",
      "category": "business" or null,
      "potential": "Could revolutionize X" or null
    }
  ],
  "notes": [
    {
      "content": "The note content",
      "tags": ["tag1", "tag2"]
    }
  ],
  "summary": "Brief 1-2 sentence overview of the brain dump contents"
}

IMPORTANT:
- Return ONLY valid JSON, no markdown code blocks or explanations
- Be thorough - don't miss any actionable items or ideas
- Use your judgment to categorize ambiguous items
- Keep the summary concise but informative"#;

#[tauri::command]
pub async fn process_brain_dump(
    app: AppHandle,
    api_key: String,
    transcript: String,
) -> Result<BrainDumpResult, String> {
    if transcript.trim().is_empty() {
        return Err("Transcript is empty. Please provide some content.".to_string());
    }

    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": format!("Process this brain dump and categorize the contents:\n\n{}", transcript)
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

    let result: BrainDumpResult = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse brain dump result: {}", e))?;

    let _ = app.emit("brain-dump-processed", &result);

    Ok(result)
}

#[tauri::command]
pub async fn process_brain_dump_streaming(
    app: AppHandle,
    api_key: String,
    transcript: String,
) -> Result<(), String> {
    if transcript.trim().is_empty() {
        return Err("Transcript is empty. Please provide some content.".to_string());
    }

    let _ = app.emit("brain-dump-started", ());

    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": format!("Process this brain dump and categorize the contents:\n\n{}", transcript)
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
                                    "brain-dump-chunk",
                                    BrainDumpChunk {
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
        "brain-dump-chunk",
        BrainDumpChunk {
            text: String::new(),
            is_complete: true,
        },
    );

    let result: BrainDumpResult = serde_json::from_str(&full_text)
        .map_err(|e| format!("Failed to parse brain dump result: {}", e))?;

    let _ = app.emit("brain-dump-complete", &result);

    Ok(())
}
