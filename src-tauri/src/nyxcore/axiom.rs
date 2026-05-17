//! nyxCore Axiom (RAG) client.
//!
//! Single endpoint: POST /api/v1/rag/search — Bearer auth token
//! stored as `nyxcore_axiom` (Sub-Project D).

use serde::{Deserialize, Serialize};

use super::client::{base_url, get_client};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomResult {
    pub content: String,
    #[serde(default)]
    pub heading: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub authority: Option<String>,
    #[serde(default)]
    pub score: f32,
    #[serde(rename = "chunkId", default)]
    pub chunk_id: Option<String>,
    #[serde(rename = "documentId", default)]
    pub document_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AxiomSearchResponse {
    #[allow(dead_code)]
    ok: bool,
    results: Vec<AxiomResult>,
}

#[derive(Debug, Serialize)]
struct AxiomSearchRequest<'a> {
    query: &'a str,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    project_id: Option<&'a str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    authority: Option<Vec<&'a str>>,
    limit: u32,
}

/// Search the Axiom knowledge base for chunks relevant to `query`.
///
/// `project_id` optionally scopes the search to a specific nyxCore
/// project; the server enforces project scope if the token itself is
/// project-scoped. `limit` is clamped to [1, 50] server-side.
#[tauri::command]
pub async fn axiom_search(
    query: String,
    project_id: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<AxiomResult>, String> {
    if query.trim().is_empty() {
        return Err("Axiom search query cannot be empty".to_string());
    }
    let limit = limit.unwrap_or(5).clamp(1, 50);

    let token = crate::secrets::get_key_or_error("nyxcore_axiom")?;
    let url = format!("{}/api/v1/rag/search", base_url());

    let body = AxiomSearchRequest {
        query: &query,
        project_id: project_id.as_deref(),
        authority: None,
        limit,
    };

    let resp = get_client()
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Axiom search failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(crate::agents::classify_api_error(resp.status()));
    }

    let parsed: AxiomSearchResponse = resp
        .json()
        .await
        .map_err(|e| format!("Axiom returned invalid JSON: {}", e))?;
    Ok(parsed.results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axiom_response_parses_with_minimal_fields() {
        let json = r#"{
            "ok": true,
            "results": [{"content":"hello","score":0.92}],
            "requestId": "abc"
        }"#;
        let r: AxiomSearchResponse = serde_json::from_str(json).expect("parse");
        assert_eq!(r.results.len(), 1);
        assert_eq!(r.results[0].content, "hello");
        assert!((r.results[0].score - 0.92).abs() < 1e-6);
    }

    #[test]
    fn axiom_response_parses_full_fields() {
        let json = r#"{
            "ok": true,
            "results": [{
                "content":"some knowledge",
                "heading":"Section 1",
                "filename":"notes.md",
                "authority":"high",
                "score":0.88,
                "chunkId":"c1",
                "documentId":"d1"
            }]
        }"#;
        let r: AxiomSearchResponse = serde_json::from_str(json).expect("parse");
        let res = &r.results[0];
        assert_eq!(res.heading.as_deref(), Some("Section 1"));
        assert_eq!(res.filename.as_deref(), Some("notes.md"));
        assert_eq!(res.chunk_id.as_deref(), Some("c1"));
    }

    #[test]
    fn axiom_request_serializes_optional_project_id_omitted() {
        let req = AxiomSearchRequest {
            query: "hello",
            project_id: None,
            authority: None,
            limit: 10,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("projectId"), "must omit project_id when None: {}", json);
        assert!(json.contains("\"query\":\"hello\""));
        assert!(json.contains("\"limit\":10"));
    }

    #[test]
    fn axiom_request_serializes_with_project_id() {
        let pid = "11111111-2222-3333-4444-555555555555";
        let req = AxiomSearchRequest {
            query: "x",
            project_id: Some(pid),
            authority: None,
            limit: 5,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(&format!("\"projectId\":\"{}\"", pid)));
    }
}
