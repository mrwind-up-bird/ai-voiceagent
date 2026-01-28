use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// In-memory cache backed by file storage
static KEYS_CACHE: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| {
    Mutex::new(load_keys_from_file().unwrap_or_default())
});

fn valid_key_types() -> Vec<&'static str> {
    vec!["deepgram", "assembly_ai", "openai", "anthropic", "qrecords"]
}

fn is_valid_key_type(key_type: &str) -> bool {
    valid_key_types().contains(&key_type)
}

fn get_keys_file_path() -> PathBuf {
    let home = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let app_dir = home.join("voice-intelligence-hub");
    fs::create_dir_all(&app_dir).ok();
    app_dir.join(".api_keys")
}

fn load_keys_from_file() -> Result<HashMap<String, String>, String> {
    let path = get_keys_file_path();
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_keys_to_file(keys: &HashMap<String, String>) -> Result<(), String> {
    let path = get_keys_file_path();
    let content = serde_json::to_string_pretty(keys).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    tracing::info!("Keys saved to {:?}", path);
    Ok(())
}

#[tauri::command]
pub async fn set_api_key(key_type: String, value: String) -> Result<(), String> {
    tracing::info!("set_api_key called for: {} (value len: {})", key_type, value.len());

    if !is_valid_key_type(&key_type) {
        tracing::warn!("Invalid key type: {}", key_type);
        return Err(format!("Unknown key type: {}", key_type));
    }

    let mut cache = KEYS_CACHE.lock().map_err(|e| e.to_string())?;
    cache.insert(key_type.clone(), value);
    save_keys_to_file(&cache)?;

    tracing::info!("API key stored: {}", key_type);
    Ok(())
}

#[tauri::command]
pub async fn get_api_key(key_type: String) -> Result<Option<String>, String> {
    if !is_valid_key_type(&key_type) {
        return Err(format!("Unknown key type: {}", key_type));
    }

    let cache = KEYS_CACHE.lock().map_err(|e| e.to_string())?;
    let result = cache.get(&key_type).cloned();

    if result.is_some() {
        tracing::info!("Found API key for: {}", key_type);
    }

    Ok(result)
}

#[tauri::command]
pub async fn delete_api_key(key_type: String) -> Result<(), String> {
    if !is_valid_key_type(&key_type) {
        return Err(format!("Unknown key type: {}", key_type));
    }

    let mut cache = KEYS_CACHE.lock().map_err(|e| e.to_string())?;
    cache.remove(&key_type);
    save_keys_to_file(&cache)?;

    tracing::info!("API key deleted: {}", key_type);
    Ok(())
}

#[tauri::command]
pub async fn has_api_keys() -> Result<bool, String> {
    let cache = KEYS_CACHE.lock().map_err(|e| e.to_string())?;
    Ok(cache.contains_key("deepgram") || cache.contains_key("assembly_ai"))
}
