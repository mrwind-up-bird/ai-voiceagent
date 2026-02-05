//! API key management with secure OS-level storage.
//!
//! This module provides secure storage for API keys using platform-specific
//! secure storage mechanisms:
//! - macOS/iOS: Security.framework Keychain
//! - Windows: Credential Manager (DPAPI)
//! - Linux: Secret Service (libsecret/GNOME Keyring)
//! - Android: EncryptedSharedPreferences with Android Keystore
//!
//! All keys are encrypted at rest using hardware-backed encryption where available.

use crate::platform::secrets::{get_storage, is_valid_key_type, SecureStorage, VALID_KEY_TYPES};

/// Store an API key securely in the system keychain/keystore
#[tauri::command]
pub async fn set_api_key(key_type: String, value: String) -> Result<(), String> {
    tracing::info!(
        "set_api_key called for: {} (value len: {})",
        key_type,
        value.len()
    );

    if !is_valid_key_type(&key_type) {
        tracing::warn!("Invalid key type: {}", key_type);
        return Err(format!(
            "Unknown key type: {}. Valid types: {:?}",
            key_type, VALID_KEY_TYPES
        ));
    }

    if value.is_empty() {
        return Err("API key value cannot be empty".to_string());
    }

    let storage = get_storage();
    storage.set(&key_type, &value)?;

    tracing::info!("API key '{}' stored in secure storage", key_type);
    Ok(())
}

/// Retrieve an API key from the system keychain/keystore
#[tauri::command]
pub async fn get_api_key(key_type: String) -> Result<Option<String>, String> {
    if !is_valid_key_type(&key_type) {
        return Err(format!(
            "Unknown key type: {}. Valid types: {:?}",
            key_type, VALID_KEY_TYPES
        ));
    }

    let storage = get_storage();
    let result = storage.get(&key_type)?;

    if result.is_some() {
        tracing::debug!("Retrieved API key for: {}", key_type);
    }

    Ok(result)
}

/// Delete an API key from the system keychain/keystore
#[tauri::command]
pub async fn delete_api_key(key_type: String) -> Result<(), String> {
    if !is_valid_key_type(&key_type) {
        return Err(format!(
            "Unknown key type: {}. Valid types: {:?}",
            key_type, VALID_KEY_TYPES
        ));
    }

    let storage = get_storage();
    storage.delete(&key_type)?;

    tracing::info!("API key '{}' deleted from secure storage", key_type);
    Ok(())
}

/// Check if any transcription API keys are configured
#[tauri::command]
pub async fn has_api_keys() -> Result<bool, String> {
    let storage = get_storage();

    let has_deepgram = storage.get("deepgram")?.is_some();
    let has_assemblyai = storage.get("assembly_ai")?.is_some();

    Ok(has_deepgram || has_assemblyai)
}

/// Get a list of all configured API key types
#[tauri::command]
pub async fn list_configured_keys() -> Result<Vec<String>, String> {
    let storage = get_storage();
    let mut configured = Vec::new();

    for key_type in VALID_KEY_TYPES {
        if storage.get(key_type)?.is_some() {
            configured.push(key_type.to_string());
        }
    }

    Ok(configured)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_key_type() {
        let result = set_api_key("invalid_type".to_string(), "value".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown key type"));
    }

    #[tokio::test]
    async fn test_empty_value() {
        let result = set_api_key("openai".to_string(), "".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }
}
