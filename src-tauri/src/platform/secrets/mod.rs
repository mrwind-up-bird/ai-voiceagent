//! Cross-platform secure credential storage.
//!
//! Platform implementations:
//! - macOS/iOS: Security.framework Keychain
//! - Windows: Credential Manager (DPAPI)
//! - Linux: Secret Service (libsecret) or encrypted file fallback
//! - Android: Android Keystore
//!
//! All implementations encrypt credentials at rest using OS-provided secure storage.

use std::fmt;

/// Error type for secure storage operations
#[derive(Debug)]
pub enum SecureStorageError {
    /// Key not found in storage
    NotFound,
    /// Platform-specific error with message
    Platform(String),
    /// Invalid key type provided
    InvalidKeyType(String),
    /// Storage is locked (e.g., keychain locked)
    Locked,
    /// Access denied (e.g., app not authorized)
    AccessDenied,
}

impl fmt::Display for SecureStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Key not found in secure storage"),
            Self::Platform(msg) => write!(f, "Platform error: {}", msg),
            Self::InvalidKeyType(key) => write!(f, "Invalid key type: {}", key),
            Self::Locked => write!(f, "Secure storage is locked"),
            Self::AccessDenied => write!(f, "Access denied to secure storage"),
        }
    }
}

impl std::error::Error for SecureStorageError {}

impl From<SecureStorageError> for String {
    fn from(err: SecureStorageError) -> Self {
        err.to_string()
    }
}

/// Trait for platform-specific secure storage implementations
pub trait SecureStorage: Send + Sync {
    /// Store a secret value
    fn set(&self, key: &str, value: &str) -> Result<(), SecureStorageError>;

    /// Retrieve a secret value
    fn get(&self, key: &str) -> Result<Option<String>, SecureStorageError>;

    /// Delete a secret
    fn delete(&self, key: &str) -> Result<(), SecureStorageError>;

    /// Check if a key exists
    fn exists(&self, key: &str) -> Result<bool, SecureStorageError> {
        Ok(self.get(key)?.is_some())
    }
}

// Platform-specific implementations
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "android")]
mod android;

// Re-export the platform-specific implementation as `PlatformStorage`
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use apple::AppleKeychain as PlatformStorage;

#[cfg(target_os = "windows")]
pub use windows::WindowsCredentialManager as PlatformStorage;

#[cfg(target_os = "linux")]
pub use linux::LinuxSecretService as PlatformStorage;

#[cfg(target_os = "android")]
pub use android::AndroidKeystore as PlatformStorage;

// Fallback for unsupported platforms (compile error)
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux",
    target_os = "android"
)))]
compile_error!("Unsupported platform for secure storage");

/// Service name used for keychain entries
pub const SERVICE_NAME: &str = "com.aurusvoiceintelligence";

/// Valid API key types
pub const VALID_KEY_TYPES: &[&str] = &["deepgram", "assembly_ai", "openai", "anthropic", "qrecords"];

/// Check if a key type is valid
pub fn is_valid_key_type(key_type: &str) -> bool {
    VALID_KEY_TYPES.contains(&key_type)
}

/// Get the default platform storage instance
pub fn get_storage() -> PlatformStorage {
    PlatformStorage::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_key_types() {
        assert!(is_valid_key_type("deepgram"));
        assert!(is_valid_key_type("openai"));
        assert!(!is_valid_key_type("invalid"));
    }
}
