use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Supported languages for translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "nl")]
    Dutch,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "zh")]
    Chinese,
    #[serde(rename = "ko")]
    Korean,
    #[serde(rename = "ar")]
    Arabic,
    #[serde(rename = "auto")]
    Auto, // Auto-detect source language
}

impl Language {
    fn code(&self) -> &str {
        match self {
            Language::English => "en",
            Language::German => "de",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::Italian => "it",
            Language::Portuguese => "pt",
            Language::Dutch => "nl",
            Language::Russian => "ru",
            Language::Japanese => "ja",
            Language::Chinese => "zh",
            Language::Korean => "ko",
            Language::Arabic => "ar",
            Language::Auto => "auto",
        }
    }

    fn name(&self) -> &str {
        match self {
            Language::English => "English",
            Language::German => "German",
            Language::Spanish => "Spanish",
            Language::French => "French",
            Language::Italian => "Italian",
            Language::Portuguese => "Portuguese",
            Language::Dutch => "Dutch",
            Language::Russian => "Russian",
            Language::Japanese => "Japanese",
            Language::Chinese => "Chinese",
            Language::Korean => "Korean",
            Language::Arabic => "Arabic",
            Language::Auto => "Auto-detect",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub original: String,
    pub translated: String,
    pub source_language: String,
    pub target_language: String,
    pub detected_language: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranslationChunk {
    pub text: String,
    pub is_complete: bool,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
}

fn build_system_prompt(source_lang: &Language, target_lang: &Language) -> String {
    let source_instruction = if matches!(source_lang, Language::Auto) {
        "Detect the source language automatically".to_string()
    } else {
        format!("The source language is {}", source_lang.name())
    };

    format!(
        r#"You are a professional translator. Your task is to translate text accurately while preserving the original meaning, tone, and style.

{}
Translate the text into {}.

Guidelines:
- Maintain the original meaning and nuance
- Preserve formatting, punctuation style appropriate for the target language
- Keep proper nouns, brand names, and technical terms as appropriate
- Adapt idioms and expressions naturally for the target language
- Do not add explanations or notes
- Respond with ONLY the translated text, nothing else"#,
        source_instruction,
        target_lang.name()
    )
}

/// Non-streaming translation
#[tauri::command]
pub async fn translate_text(
    app: AppHandle,
    api_key: String,
    text: String,
    source_language: Language,
    target_language: Language,
) -> Result<TranslationResult, String> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "content": build_system_prompt(&source_language, &target_language)
            },
            {
                "role": "user",
                "content": text
            }
        ],
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

    let translated_text = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let result = TranslationResult {
        original: text,
        translated: translated_text,
        source_language: source_language.code().to_string(),
        target_language: target_language.code().to_string(),
        detected_language: if matches!(source_language, Language::Auto) {
            Some("auto-detected".to_string())
        } else {
            None
        },
    };

    let _ = app.emit("translation-complete", &result);

    Ok(result)
}

/// Streaming translation with real-time chunks
#[tauri::command]
pub async fn translate_text_streaming(
    app: AppHandle,
    api_key: String,
    text: String,
    source_language: Language,
    target_language: Language,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "content": build_system_prompt(&source_language, &target_language)
            },
            {
                "role": "user",
                "content": text
            }
        ],
        "temperature": 0.3,
        "stream": true
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

    let _ = app.emit("translation-started", ());

    let mut stream = response.bytes_stream();
    let mut full_text = String::new();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        // Parse SSE events (data: {...}\n\n format)
        while let Some(event_end) = buffer.find("\n\n") {
            let event_str = buffer[..event_end].to_string();
            buffer = buffer[event_end + 2..].to_string();

            for line in event_str.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(chunk_data) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                        if let Some(choice) = chunk_data.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                full_text.push_str(content);
                                let _ = app.emit(
                                    "translation-chunk",
                                    TranslationChunk {
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

    // Emit completion marker
    let _ = app.emit(
        "translation-chunk",
        TranslationChunk {
            text: String::new(),
            is_complete: true,
        },
    );

    let result = TranslationResult {
        original: text,
        translated: full_text,
        source_language: source_language.code().to_string(),
        target_language: target_language.code().to_string(),
        detected_language: if matches!(source_language, Language::Auto) {
            Some("auto-detected".to_string())
        } else {
            None
        },
    };

    let _ = app.emit("translation-complete", &result);

    Ok(())
}

/// Get list of available languages for translation
#[tauri::command]
pub fn get_available_languages() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({ "code": "auto", "name": "Auto-detect", "isSource": true }),
        serde_json::json!({ "code": "en", "name": "English", "isSource": true }),
        serde_json::json!({ "code": "de", "name": "German", "isSource": true }),
        serde_json::json!({ "code": "es", "name": "Spanish", "isSource": true }),
        serde_json::json!({ "code": "fr", "name": "French", "isSource": true }),
        serde_json::json!({ "code": "it", "name": "Italian", "isSource": true }),
        serde_json::json!({ "code": "pt", "name": "Portuguese", "isSource": true }),
        serde_json::json!({ "code": "nl", "name": "Dutch", "isSource": true }),
        serde_json::json!({ "code": "ru", "name": "Russian", "isSource": true }),
        serde_json::json!({ "code": "ja", "name": "Japanese", "isSource": true }),
        serde_json::json!({ "code": "zh", "name": "Chinese", "isSource": true }),
        serde_json::json!({ "code": "ko", "name": "Korean", "isSource": true }),
        serde_json::json!({ "code": "ar", "name": "Arabic", "isSource": true }),
    ]
}
