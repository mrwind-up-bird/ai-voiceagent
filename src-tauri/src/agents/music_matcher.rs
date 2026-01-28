use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

// Q-Records API endpoint (placeholder - replace with actual endpoint)
const QRECORDS_API_URL: &str = "https://api.qrecords.com/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicMatchRequest {
    pub query: String,
    pub mood: Option<String>,
    pub genre: Option<String>,
    pub tempo: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: u32,
    pub preview_url: Option<String>,
    pub cover_art_url: Option<String>,
    pub match_score: f32,
    pub mood_tags: Vec<String>,
    pub genre_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicMatchResult {
    pub query: String,
    pub tracks: Vec<Track>,
    pub analysis: MoodAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodAnalysis {
    pub detected_mood: String,
    pub energy_level: f32,
    pub valence: f32,
    pub keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct QRecordsResponse {
    tracks: Vec<QRecordsTrack>,
    mood_analysis: Option<QRecordsMoodAnalysis>,
}

#[derive(Debug, Deserialize)]
struct QRecordsTrack {
    id: String,
    title: String,
    artist: String,
    album: Option<String>,
    duration_ms: u32,
    preview_url: Option<String>,
    cover_art_url: Option<String>,
    relevance_score: f32,
    mood: Vec<String>,
    genre: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct QRecordsMoodAnalysis {
    mood: String,
    energy: f32,
    valence: f32,
    keywords: Vec<String>,
}

#[tauri::command]
pub async fn match_music(
    app: AppHandle,
    api_key: String,
    request: MusicMatchRequest,
) -> Result<MusicMatchResult, String> {
    let client = reqwest::Client::new();

    let mut query_params = vec![("query", request.query.clone())];

    if let Some(mood) = &request.mood {
        query_params.push(("mood", mood.clone()));
    }
    if let Some(genre) = &request.genre {
        query_params.push(("genre", genre.clone()));
    }
    if let Some(tempo) = &request.tempo {
        query_params.push(("tempo", tempo.clone()));
    }

    let limit = request.limit.unwrap_or(10);
    query_params.push(("limit", limit.to_string()));

    let response = client
        .get(format!("{}/search", QRECORDS_API_URL))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .query(&query_params)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        // If Q-Records API fails, return mock data for development
        tracing::warn!("Q-Records API unavailable, returning mock data");
        return Ok(create_mock_result(&request.query));
    }

    let qrecords_response: QRecordsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let tracks: Vec<Track> = qrecords_response
        .tracks
        .into_iter()
        .map(|t| Track {
            id: t.id,
            title: t.title,
            artist: t.artist,
            album: t.album,
            duration_ms: t.duration_ms,
            preview_url: t.preview_url,
            cover_art_url: t.cover_art_url,
            match_score: t.relevance_score,
            mood_tags: t.mood,
            genre_tags: t.genre,
        })
        .collect();

    let analysis = qrecords_response
        .mood_analysis
        .map(|ma| MoodAnalysis {
            detected_mood: ma.mood,
            energy_level: ma.energy,
            valence: ma.valence,
            keywords: ma.keywords,
        })
        .unwrap_or(MoodAnalysis {
            detected_mood: "neutral".to_string(),
            energy_level: 0.5,
            valence: 0.5,
            keywords: vec![],
        });

    let result = MusicMatchResult {
        query: request.query,
        tracks,
        analysis,
    };

    let _ = app.emit("music-matched", &result);

    Ok(result)
}

#[tauri::command]
pub async fn analyze_mood_from_transcript(
    app: AppHandle,
    openai_key: String,
    transcript: String,
) -> Result<MoodAnalysis, String> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": r#"Analyze the emotional mood and energy of the given text. Return a JSON object with:
{
  "detected_mood": "primary mood (e.g., happy, sad, energetic, calm, anxious, hopeful)",
  "energy_level": 0.0-1.0 (0 = very low energy, 1 = very high energy),
  "valence": 0.0-1.0 (0 = very negative, 1 = very positive),
  "keywords": ["array", "of", "mood", "keywords"]
}
Only return the JSON object, no other text."#
            },
            {
                "role": "user",
                "content": transcript
            }
        ],
        "response_format": { "type": "json_object" }
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_key))
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

    let analysis: MoodAnalysis = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse mood analysis: {}", e))?;

    let _ = app.emit("mood-analyzed", &analysis);

    Ok(analysis)
}

fn create_mock_result(query: &str) -> MusicMatchResult {
    MusicMatchResult {
        query: query.to_string(),
        tracks: vec![
            Track {
                id: "mock-1".to_string(),
                title: "Calm Waters".to_string(),
                artist: "Ambient Dreams".to_string(),
                album: Some("Peaceful Moments".to_string()),
                duration_ms: 240000,
                preview_url: None,
                cover_art_url: None,
                match_score: 0.95,
                mood_tags: vec!["calm".to_string(), "peaceful".to_string()],
                genre_tags: vec!["ambient".to_string(), "instrumental".to_string()],
            },
            Track {
                id: "mock-2".to_string(),
                title: "Morning Light".to_string(),
                artist: "Sunrise Collective".to_string(),
                album: Some("New Beginnings".to_string()),
                duration_ms: 195000,
                preview_url: None,
                cover_art_url: None,
                match_score: 0.88,
                mood_tags: vec!["hopeful".to_string(), "uplifting".to_string()],
                genre_tags: vec!["indie".to_string(), "folk".to_string()],
            },
        ],
        analysis: MoodAnalysis {
            detected_mood: "neutral".to_string(),
            energy_level: 0.5,
            valence: 0.6,
            keywords: vec!["query".to_string(), "based".to_string()],
        },
    }
}

#[tauri::command]
pub fn get_available_moods() -> Vec<String> {
    vec![
        "happy".to_string(),
        "sad".to_string(),
        "energetic".to_string(),
        "calm".to_string(),
        "anxious".to_string(),
        "hopeful".to_string(),
        "melancholic".to_string(),
        "romantic".to_string(),
        "angry".to_string(),
        "peaceful".to_string(),
    ]
}

#[tauri::command]
pub fn get_available_genres() -> Vec<String> {
    vec![
        "pop".to_string(),
        "rock".to_string(),
        "electronic".to_string(),
        "classical".to_string(),
        "jazz".to_string(),
        "hip-hop".to_string(),
        "r&b".to_string(),
        "country".to_string(),
        "folk".to_string(),
        "ambient".to_string(),
        "indie".to_string(),
        "metal".to_string(),
    ]
}
