//! Linux Secret Service implementation
//!
//! Uses the freedesktop.org Secret Service API (libsecret) to securely store API keys.
//! Falls back to encrypted file storage if Secret Service is unavailable.
//!
//! Supports:
//! - GNOME Keyring
//! - KDE Wallet (via Secret Service portal)
//! - Other Secret Service implementations

use secret_service::{EncryptionType, SecretService};
use std::collections::HashMap;

use super::{SecureStorage, SecureStorageError, SERVICE_NAME};

/// Linux Secret Service secure storage implementation
pub struct LinuxSecretService {
    service_name: String,
}

impl LinuxSecretService {
    /// Create a new Secret Service storage instance
    pub fn new() -> Self {
        Self {
            service_name: SERVICE_NAME.to_string(),
        }
    }

    /// Create with a custom service name (for testing)
    #[allow(dead_code)]
    pub fn with_service(service: &str) -> Self {
        Self {
            service_name: service.to_string(),
        }
    }

    /// Get the Secret Service connection
    fn get_service(&self) -> Result<SecretService<'static>, SecureStorageError> {
        SecretService::connect(EncryptionType::Dh)
            .map_err(|e| SecureStorageError::Platform(format!("Failed to connect to Secret Service: {}", e)))
    }

    /// Get or create the collection for this app
    fn get_collection<'a>(&self, service: &'a SecretService<'a>) -> Result<secret_service::Collection<'a>, SecureStorageError> {
        // Try to get the default collection first
        let collection = service
            .get_default_collection()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get default collection: {}", e)))?;

        // Unlock if necessary
        if collection.is_locked().unwrap_or(true) {
            collection
                .unlock()
                .map_err(|e| SecureStorageError::Platform(format!("Failed to unlock collection: {}", e)))?;
        }

        Ok(collection)
    }

    /// Build attributes for an item
    fn build_attributes(&self, key: &str) -> HashMap<&str, &str> {
        let mut attrs = HashMap::new();
        attrs.insert("application", "aurus-voice-intelligence");
        attrs.insert("key", key);
        attrs
    }
}

impl Default for LinuxSecretService {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for LinuxSecretService {
    fn set(&self, key: &str, value: &str) -> Result<(), SecureStorageError> {
        let service = self.get_service()?;
        let collection = self.get_collection(&service)?;

        let label = format!("{}: {}", self.service_name, key);
        let attributes: Vec<(&str, &str)> = vec![
            ("application", "aurus-voice-intelligence"),
            ("key", key),
        ];

        // Delete existing item first
        let _ = self.delete(key);

        collection
            .create_item(
                &label,
                attributes,
                value.as_bytes(),
                true, // replace if exists
                "text/plain",
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create secret: {}", e)))?;

        tracing::info!("Stored key '{}' in Linux Secret Service", key);
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, SecureStorageError> {
        let service = self.get_service()?;
        let collection = self.get_collection(&service)?;

        let attrs = self.build_attributes(key);
        let search_attrs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, *v)).collect();

        let items = collection
            .search_items(search_attrs)
            .map_err(|e| SecureStorageError::Platform(format!("Failed to search secrets: {}", e)))?;

        if let Some(item) = items.first() {
            // Unlock item if necessary
            if item.is_locked().unwrap_or(true) {
                item.unlock()
                    .map_err(|e| SecureStorageError::Platform(format!("Failed to unlock item: {}", e)))?;
            }

            let secret = item
                .get_secret()
                .map_err(|e| SecureStorageError::Platform(format!("Failed to get secret: {}", e)))?;

            let value = String::from_utf8(secret)
                .map_err(|e| SecureStorageError::Platform(format!("Invalid UTF-8: {}", e)))?;

            tracing::debug!("Retrieved key '{}' from Secret Service", key);
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecureStorageError> {
        let service = self.get_service()?;
        let collection = self.get_collection(&service)?;

        let attrs = self.build_attributes(key);
        let search_attrs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, *v)).collect();

        let items = collection
            .search_items(search_attrs)
            .map_err(|e| SecureStorageError::Platform(format!("Failed to search secrets: {}", e)))?;

        for item in items {
            item.delete()
                .map_err(|e| SecureStorageError::Platform(format!("Failed to delete secret: {}", e)))?;
        }

        tracing::info!("Deleted key '{}' from Secret Service", key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires D-Bus and Secret Service daemon
    fn test_secret_service_roundtrip() {
        let storage = LinuxSecretService::with_service("com.aurusvoiceintelligence.test");
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
