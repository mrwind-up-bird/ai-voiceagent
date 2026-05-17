//! Persona Studio client.
//!
//! Endpoints consumed:
//!   - GET  /api/v1/persona/list   — circles + personas the token can use
//!   - POST /api/v1/persona/chat   — persona-tuned reply for arbitrary messages
//!
//! Auth: Bearer `nyx_pa_…` token, stored in keychain under
//! `persona_studio` (see Sub-Project D).

use serde::{Deserialize, Serialize};

use super::client::{base_url, get_client};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaProfile {
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub limbic_type: Option<String>,
    #[serde(default)]
    pub elevator_pitch: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaSummary {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub is_lead: bool,
    #[serde(default)]
    pub profile: Option<PersonaProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaCircle {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub tagline: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub personas: Vec<PersonaSummary>,
}

#[derive(Debug, Deserialize)]
struct PersonaListResponse {
    #[allow(dead_code)]
    ok: bool,
    circles: Vec<PersonaCircle>,
}

/// List the persona circles + personas the configured token can access.
#[tauri::command]
pub async fn list_personas() -> Result<Vec<PersonaCircle>, String> {
    let token = crate::secrets::get_key_or_error("persona_studio")?;
    let url = format!("{}/api/v1/persona/list", base_url());
    let resp = get_client()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Persona Studio request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(crate::agents::classify_api_error(resp.status()));
    }
    let body: PersonaListResponse = resp
        .json()
        .await
        .map_err(|e| format!("Persona Studio returned invalid JSON: {}", e))?;
    Ok(body.circles)
}

#[derive(Debug, Serialize)]
struct PersonaChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct PersonaChatRequest<'a> {
    messages: Vec<PersonaChatMessage<'a>>,
    #[serde(rename = "personaId", skip_serializing_if = "Option::is_none")]
    persona_id: Option<&'a str>,
    #[serde(rename = "circleId", skip_serializing_if = "Option::is_none")]
    circle_id: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct PersonaChatResponse {
    #[allow(dead_code)]
    ok: bool,
    #[serde(default)]
    reply: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

impl PersonaChatResponse {
    fn into_text(self) -> Option<String> {
        self.reply.or(self.text).or(self.content)
    }
}

/// Rephrase `text` through the configured persona's voice.
///
/// Used by Sub-Project E + F flows to give the user persona-tuned
/// todos / mental-mirror letters / brain-dumps without duplicating
/// the tone-shifter agent. Either `persona_id` OR `circle_id` must
/// be provided.
#[tauri::command]
pub async fn apply_persona_tone(
    text: String,
    persona_id: Option<String>,
    circle_id: Option<String>,
) -> Result<String, String> {
    if persona_id.is_none() && circle_id.is_none() {
        return Err("Specify either persona_id or circle_id".to_string());
    }
    if text.trim().is_empty() {
        return Err("Nothing to rephrase (empty text)".to_string());
    }

    let token = crate::secrets::get_key_or_error("persona_studio")?;
    let url = format!("{}/api/v1/persona/chat", base_url());

    let user_prompt = format!(
        "Reword the following in your authentic voice, preserving meaning, length, and tone constraints. \
         Return only the rewritten text — no preamble, no surrounding quotes.\n\n{}",
        text
    );

    let body = PersonaChatRequest {
        messages: vec![PersonaChatMessage {
            role: "user",
            content: &user_prompt,
        }],
        persona_id: persona_id.as_deref(),
        circle_id: circle_id.as_deref(),
    };

    let resp = get_client()
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Persona Studio chat failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(crate::agents::classify_api_error(resp.status()));
    }

    let parsed: PersonaChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("Persona Studio returned invalid JSON: {}", e))?;
    parsed
        .into_text()
        .ok_or_else(|| "Persona Studio returned no content".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persona_list_response_parses_with_minimal_fields() {
        let json = r#"{
            "ok": true,
            "circles": [
                {"id":"c1","slug":"founders","name":"Founders","personas":[]}
            ],
            "requestId": "abc"
        }"#;
        let r: PersonaListResponse = serde_json::from_str(json).expect("parse");
        assert_eq!(r.circles.len(), 1);
        assert_eq!(r.circles[0].slug, "founders");
    }

    #[test]
    fn persona_list_response_tolerates_missing_optional_fields() {
        let json = r#"{
            "ok": true,
            "circles": [{
                "id":"c1","slug":"f","name":"F",
                "personas":[
                    {"id":"p1","name":"Riley"}
                ]
            }]
        }"#;
        let r: PersonaListResponse = serde_json::from_str(json).expect("parse");
        let p = &r.circles[0].personas[0];
        assert_eq!(p.name, "Riley");
        assert!(p.description.is_none());
        assert!(p.image_url.is_none());
        assert!(!p.is_lead);
    }

    #[test]
    fn chat_response_into_text_prefers_reply() {
        let r = PersonaChatResponse {
            ok: true,
            reply: Some("from reply".into()),
            text: Some("from text".into()),
            content: Some("from content".into()),
        };
        assert_eq!(r.into_text().as_deref(), Some("from reply"));
    }

    #[test]
    fn chat_response_into_text_falls_back_to_text_then_content() {
        let r = PersonaChatResponse {
            ok: true,
            reply: None,
            text: Some("from text".into()),
            content: Some("from content".into()),
        };
        assert_eq!(r.into_text().as_deref(), Some("from text"));

        let r = PersonaChatResponse {
            ok: true,
            reply: None,
            text: None,
            content: Some("from content".into()),
        };
        assert_eq!(r.into_text().as_deref(), Some("from content"));
    }

    #[test]
    fn chat_response_into_text_none_when_all_empty() {
        let r = PersonaChatResponse {
            ok: true,
            reply: None,
            text: None,
            content: None,
        };
        assert!(r.into_text().is_none());
    }
}
