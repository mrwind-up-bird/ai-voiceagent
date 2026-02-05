//! macOS and iOS Keychain implementation using Security.framework
//!
//! Uses the system Keychain to securely store API keys with:
//! - Hardware-backed encryption on devices with Secure Enclave
//! - Automatic iCloud Keychain sync (if enabled by user)
//! - App-specific access control

use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use super::{SecureStorage, SecureStorageError, SERVICE_NAME};

/// Apple Keychain secure storage implementation
pub struct AppleKeychain {
    service: String,
}

impl AppleKeychain {
    /// Create a new Keychain storage instance
    pub fn new() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
        }
    }

    /// Create with a custom service name (for testing)
    #[allow(dead_code)]
    pub fn with_service(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }
}

impl Default for AppleKeychain {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for AppleKeychain {
    fn set(&self, key: &str, value: &str) -> Result<(), SecureStorageError> {
        // Delete existing entry first (update not directly supported)
        let _ = delete_generic_password(&self.service, key);

        set_generic_password(&self.service, key, value.as_bytes()).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("denied") || msg.contains("authorized") {
                SecureStorageError::AccessDenied
            } else {
                SecureStorageError::Platform(msg)
            }
        })?;

        tracing::info!("Stored key '{}' in macOS/iOS Keychain", key);
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, SecureStorageError> {
        match get_generic_password(&self.service, key) {
            Ok(bytes) => {
                let value = String::from_utf8(bytes).map_err(|e| {
                    SecureStorageError::Platform(format!("Invalid UTF-8 in keychain: {}", e))
                })?;
                tracing::debug!("Retrieved key '{}' from Keychain", key);
                Ok(Some(value))
            }
            Err(e) => {
                let msg = e.to_string();
                // Item not found is not an error, just means key doesn't exist
                // Handle various "not found" message formats from Security.framework
                if msg.contains("not found")
                    || msg.contains("could not be found")
                    || msg.contains("-25300")
                {
                    Ok(None)
                } else if msg.contains("denied") || msg.contains("authorized") {
                    Err(SecureStorageError::AccessDenied)
                } else {
                    Err(SecureStorageError::Platform(msg))
                }
            }
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecureStorageError> {
        match delete_generic_password(&self.service, key) {
            Ok(()) => {
                tracing::info!("Deleted key '{}' from Keychain", key);
                Ok(())
            }
            Err(e) => {
                let msg = e.to_string();
                // Ignore "not found" errors when deleting
                if msg.contains("not found")
                    || msg.contains("could not be found")
                    || msg.contains("-25300")
                {
                    Ok(())
                } else if msg.contains("denied") || msg.contains("authorized") {
                    Err(SecureStorageError::AccessDenied)
                } else {
                    Err(SecureStorageError::Platform(msg))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_roundtrip() {
        let storage = AppleKeychain::with_service("com.aurusvoiceintelligence.test");
        let key = "test_api_key";
        let value = "sk-test-12345";

        // Clean up any existing test data
        let _ = storage.delete(key);

        // Set and get
        storage.set(key, value).expect("Failed to set");
        let retrieved = storage.get(key).expect("Failed to get");
        assert_eq!(retrieved, Some(value.to_string()));

        // Delete
        storage.delete(key).expect("Failed to delete");
        let after_delete = storage.get(key).expect("Failed to get after delete");
        assert_eq!(after_delete, None);
    }
}
