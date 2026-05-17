use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MAX_TRANSCRIPT_LENGTH: usize = 100_000; // ~100KB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    // H8: tolerate API drift / partial generations — these fields used
    // to be required (`String`), and a model omitting `priority` would
    // leak a raw serde error to the UI with transcript snippets. Now
    // missing fields default to empty/lowercase-default values.
    #[serde(default)]
    pub task: String,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: String,
    pub context: Option<String>,
    /// Sub-Project F — coarse category for visual grouping in the UI.
    /// Free-text (model-supplied) — typical values: "work", "personal",
    /// "errand", "follow-up", "decision", "research". None when the
    /// model can't infer one.
    #[serde(default)]
    pub category: Option<String>,
    /// Sub-Project F — human-friendly explanation for the assigned
    /// priority + suggested due date so the user knows WHY the model
    /// suggested what it did. Empty when the model didn't generate a
    /// rationale.
    #[serde(default)]
    pub rationale: Option<String>,
}

fn default_priority() -> String {
    "medium".to_string()
}

/// Sub-Project F — normalise a priority string to one of the three
/// canonical values used by the UI styling. Anything unexpected
/// degrades to "medium" so the model never crashes the chip palette
/// with creative spellings like "urgent" or "low-key".
pub fn normalize_priority(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "high" | "urgent" | "critical" | "p0" | "p1" => "high",
        "low" | "trivial" | "p3" | "p4" => "low",
        _ => "medium",
    }
}

/// Sub-Project F — normalise a category to a small canonical set so
/// the UI grouping doesn't get polluted with synonyms. Uses whole-word
/// match (split on non-alphanumeric) to avoid substring traps like
/// "framework" containing "work". Checks more-specific buckets first
/// so a phrase like "decide on framework" lands in `decision`, not
/// `work`.
pub fn normalize_category(raw: &str) -> &'static str {
    let lower = raw.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |needles: &[&str]| words.iter().any(|w| needles.contains(w));

    // Order matters — most distinctive buckets first.
    if has(&["follow", "followup", "reply", "response", "respond"]) {
        "follow-up"
    } else if has(&["decision", "decide", "decided", "choose", "choice"]) {
        "decision"
    } else if has(&["research", "learn", "study", "investigate", "explore"]) {
        "research"
    } else if has(&["errand", "shopping", "buy", "pick", "groceries"]) {
        "errand"
    } else if has(&["work", "job", "office", "client", "meeting"]) {
        "work"
    } else if has(&["personal", "self", "home", "family"]) {
        "personal"
    } else {
        "other"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItemsResult {
    #[serde(default)]
    pub items: Vec<ActionItem>,
    #[serde(default)]
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
      "task": "Description of the action item — clear and actionable",
      "assignee": "Person responsible (or null if not specified)",
      "due_date": "Concrete date like 2026-05-24 or null. If the speaker says 'this week' / 'next Monday' / 'soon', infer a concrete ISO date relative to today and put your reasoning in `rationale`.",
      "priority": "high|medium|low. Use 'high' for things with explicit urgency or hard deadlines, 'low' for nice-to-have or vague intents, otherwise 'medium'.",
      "category": "One of: work, personal, errand, follow-up, decision, research, other. Pick the closest fit from the conversational context.",
      "context": "Brief context from the conversation — one sentence",
      "rationale": "Why this priority + due date. ONE sentence, no preamble. Empty string if obvious."
    }
  ],
  "summary": "Brief summary of the key takeaways (1-3 sentences)"
}

Be thorough but precise. Only include clear action items, not general discussion points.
The `due_date` field MUST either be null or a concrete ISO date — never vague phrases like 'soon'.
The `rationale` field gives the user agency: it should explain your priority/deadline call in plain language."#;

#[tauri::command]
pub async fn extract_action_items(
    app: AppHandle,
    transcript: String,
) -> Result<ActionItemsResult, String> {
    if transcript.len() > MAX_TRANSCRIPT_LENGTH {
        return Err(format!("Transcript too long ({} chars, max {})", transcript.len(), MAX_TRANSCRIPT_LENGTH));
    }
    let api_key = crate::secrets::get_key_or_error("openai")?;
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
        .map_err(|e| {
            tracing::error!("Action items API request failed: {}", e);
            "Service temporarily unavailable. Please try again.".to_string()
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!("Action items API error: {}", error_text);
        return Err(super::classify_api_error(status));
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
    transcript: String,
) -> Result<(), String> {
    if transcript.len() > MAX_TRANSCRIPT_LENGTH {
        return Err(format!("Transcript too long ({} chars, max {})", transcript.len(), MAX_TRANSCRIPT_LENGTH));
    }
    // For streaming, we'll use the non-streaming endpoint but emit progress
    let _ = app.emit("action-items-processing", ());

    let result = extract_action_items(app.clone(), transcript).await?;

    let _ = app.emit("action-items-complete", &result);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // H8 — ActionItemsResult must tolerate API schema drift without
    // leaking raw serde errors (which previously included transcript
    // snippets) to the UI.

    #[test]
    fn action_items_result_parses_with_missing_priority() {
        let json = r#"{"items":[{"task":"buy milk"}], "summary":"ok"}"#;
        let r: ActionItemsResult = serde_json::from_str(json).expect("parse");
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.items[0].priority, "medium");
    }

    #[test]
    fn action_items_result_parses_with_missing_task() {
        let json = r#"{"items":[{"priority":"high"}], "summary":"ok"}"#;
        let r: ActionItemsResult = serde_json::from_str(json).expect("parse");
        assert_eq!(r.items[0].task, "");
        assert_eq!(r.items[0].priority, "high");
    }

    #[test]
    fn action_items_result_parses_empty_object() {
        let json = "{}";
        let r: ActionItemsResult = serde_json::from_str(json).expect("parse");
        assert!(r.items.is_empty());
        assert_eq!(r.summary, "");
    }

    #[test]
    fn action_items_result_parses_with_only_summary() {
        let json = r#"{"summary":"no actionable items"}"#;
        let r: ActionItemsResult = serde_json::from_str(json).expect("parse");
        assert!(r.items.is_empty());
        assert_eq!(r.summary, "no actionable items");
    }

    // Sub-Project F — priority + category normalisation

    #[test]
    fn normalize_priority_canonicalises_synonyms() {
        assert_eq!(super::normalize_priority("URGENT"), "high");
        assert_eq!(super::normalize_priority("critical"), "high");
        assert_eq!(super::normalize_priority("P0"), "high");
        assert_eq!(super::normalize_priority("p1"), "high");
        assert_eq!(super::normalize_priority("low"), "low");
        assert_eq!(super::normalize_priority("trivial"), "low");
        assert_eq!(super::normalize_priority("medium"), "medium");
    }

    #[test]
    fn normalize_priority_unknown_degrades_to_medium() {
        assert_eq!(super::normalize_priority("low-key"), "medium");
        assert_eq!(super::normalize_priority("sometime"), "medium");
        assert_eq!(super::normalize_priority(""), "medium");
        assert_eq!(super::normalize_priority("🔥🔥🔥"), "medium");
    }

    #[test]
    fn normalize_category_maps_synonyms_to_buckets() {
        assert_eq!(super::normalize_category("work"), "work");
        assert_eq!(super::normalize_category("Office task"), "work");
        assert_eq!(super::normalize_category("personal goal"), "personal");
        assert_eq!(super::normalize_category("Home repair"), "personal");
        assert_eq!(super::normalize_category("shopping"), "errand");
        assert_eq!(super::normalize_category("buy milk"), "errand");
        assert_eq!(super::normalize_category("Follow up on email"), "follow-up");
        assert_eq!(super::normalize_category("Need to decide on framework"), "decision");
        assert_eq!(super::normalize_category("Research libraries"), "research");
    }

    #[test]
    fn normalize_category_unknown_is_other() {
        assert_eq!(super::normalize_category("travel"), "other");
        assert_eq!(super::normalize_category(""), "other");
    }

    #[test]
    fn action_item_parses_with_category_and_rationale() {
        let json = r#"{
            "task":"send invoice",
            "priority":"high",
            "category":"work",
            "rationale":"Client mentioned end-of-month deadline twice."
        }"#;
        let item: ActionItem = serde_json::from_str(json).expect("parse");
        assert_eq!(item.category.as_deref(), Some("work"));
        assert!(item.rationale.as_deref().unwrap().contains("deadline"));
    }

    #[test]
    fn action_item_parses_without_category_or_rationale() {
        let json = r#"{"task":"x","priority":"low"}"#;
        let item: ActionItem = serde_json::from_str(json).expect("parse");
        assert!(item.category.is_none());
        assert!(item.rationale.is_none());
    }
}
